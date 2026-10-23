use std::io::{Read, Result as IoResult, Write};
use std::net::TcpListener;
use std::process::{Command, Stdio};
use std::sync::{Arc, Mutex, Once, OnceLock};
use std::thread;

use async_trait::async_trait;
use chrono::Utc;
use diesel::{Connection, PgConnection};
use diesel_migrations::{embed_migrations, EmbeddedMigrations, MigrationHarness};
use jsonwebtoken::{Algorithm, EncodingKey, Header};
use rocket::http::{Header as HttpHeader, Status};
use rocket::local::asynchronous::Client;
use rocket::{catchers, routes};
use serde_json::{json, Value};
use tracing::subscriber::DefaultGuard;
use tracing_subscriber::fmt::MakeWriter;
use uuid::Uuid;

use kinetix_pricing_service::clients::{
    MerchantDirectoryPort, MerchantStanding, PeerError, ProductDirectoryPort,
};
use kinetix_pricing_service::db::create_pool;
use kinetix_pricing_service::observability::admin_actions::{
    admin_action_of_route, DISCOUNT_CREATE, FLASH_SALE_CREATE, KNOWN_ADMIN_ACTIONS, VOUCHER_APPLY,
    VOUCHER_CREATE, VOUCHER_READ,
};
use kinetix_pricing_service::observability::{json_logging, AdminAuditFairing, Metrics};
use kinetix_pricing_service::routes::catchers::{internal_error, not_found, other, unprocessable};
use kinetix_pricing_service::routes::discount_routes::{create_discount, list_discounts};
use kinetix_pricing_service::routes::flash_sale_routes::{
    create_flash_sale, get_flash_sale_for_product,
};
use kinetix_pricing_service::routes::voucher_routes::{apply_voucher, create_voucher, get_voucher};
use kinetix_pricing_service::security::jwt::JwtVerifier;
use kinetix_pricing_service::services::PromotionService;
use kinetix_pricing_service::DbPool;

const MIGRATIONS: EmbeddedMigrations = embed_migrations!("migrations");
const DATABASE_VARIABLE: &str = "KINETIX_TEST_POSTGRES";
const SIGNING_KEY_ID: &str = "admin-audit-test";
const ISSUER: &str = "https://identity.test";
const AUDIENCE: &str = "kinetix";
const MERCHANT: &str = "merchant-principal-identity-names";
const OUTCOMES: [&str; 3] = ["done", "refused", "failed"];

static MIGRATED: Once = Once::new();
static IDENTITY_CONFIGURED: Once = Once::new();

fn database_url() -> Option<String> {
    if let Ok(url) = std::env::var(DATABASE_VARIABLE) {
        MIGRATED.call_once(|| {
            let mut connection =
                PgConnection::establish(&url).expect("the test database is not reachable");
            connection
                .run_pending_migrations(MIGRATIONS)
                .expect("the migrations did not apply");
        });
        return Some(url);
    }

    assert!(
        std::env::var("CI").is_err(),
        "CI must set {DATABASE_VARIABLE}; without it these tests prove nothing"
    );
    return None;
}

struct SigningKey {
    pem: String,
    modulus: String,
}

fn openssl(args: &[&str], input: &str) -> String {
    let mut child = Command::new("openssl")
        .args(args)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("these tests sign their tokens with a key the openssl command generates");

    let mut stdin = child.stdin.take().expect("openssl took no input");
    stdin
        .write_all(input.as_bytes())
        .expect("openssl refused its input");
    drop(stdin);

    let output = child.wait_with_output().expect("openssl did not finish");
    assert!(
        output.status.success(),
        "openssl {args:?} failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    return String::from_utf8(output.stdout).expect("openssl wrote something other than text");
}

fn base64url(bytes: &[u8]) -> String {
    const ALPHABET: &[u8] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789-_";
    let mut out = String::new();
    for chunk in bytes.chunks(3) {
        let b = [
            chunk[0],
            *chunk.get(1).unwrap_or(&0),
            *chunk.get(2).unwrap_or(&0),
        ];
        let n = (u32::from(b[0]) << 16) | (u32::from(b[1]) << 8) | u32::from(b[2]);
        for i in 0..chunk.len() + 1 {
            out.push(char::from(ALPHABET[((n >> (18 - 6 * i)) & 63) as usize]));
        }
    }
    return out;
}

fn signing_key() -> &'static SigningKey {
    static KEY: OnceLock<SigningKey> = OnceLock::new();
    return KEY.get_or_init(|| {
        let pem = openssl(
            &[
                "genpkey",
                "-algorithm",
                "RSA",
                "-pkeyopt",
                "rsa_keygen_bits:2048",
            ],
            "",
        );
        let printed = openssl(&["rsa", "-noout", "-modulus"], &pem);
        let hex = printed
            .trim()
            .strip_prefix("Modulus=")
            .expect("openssl did not print the modulus");
        let bytes: Vec<u8> = (0..hex.len())
            .step_by(2)
            .map(|at| return u8::from_str_radix(&hex[at..at + 2], 16).expect("a hex byte"))
            .collect();

        return SigningKey {
            pem,
            modulus: base64url(&bytes),
        };
    });
}

fn identity_jwks_url() -> String {
    let jwks = json!({
        "keys": [{
            "kty": "RSA",
            "kid": SIGNING_KEY_ID,
            "alg": "RS256",
            "use": "sig",
            "n": signing_key().modulus,
            "e": "AQAB",
        }]
    })
    .to_string();
    let listener = TcpListener::bind("127.0.0.1:0").expect("no port for the JWKS");
    let address = listener.local_addr().expect("the JWKS has no address");

    thread::spawn(move || {
        for stream in listener.incoming() {
            let Ok(mut stream) = stream else {
                continue;
            };
            let mut request = [0_u8; 4096];
            let _ = stream.read(&mut request);
            let response = format!(
                "HTTP/1.1 200 OK\r\ncontent-type: application/json\r\ncontent-length: {}\r\nconnection: close\r\n\r\n{}",
                jwks.len(),
                jwks
            );
            let _ = stream.write_all(response.as_bytes());
        }
    });

    return format!("http://{address}/.well-known/jwks.json");
}

fn verifier() -> JwtVerifier {
    IDENTITY_CONFIGURED.call_once(|| {
        std::env::set_var("IDENTITY_JWKS_URL", identity_jwks_url());
        std::env::set_var("JWT_ISSUER", ISSUER);
        std::env::set_var("JWT_AUDIENCE", AUDIENCE);
    });
    return JwtVerifier::from_env().expect("the verifier reads its three variables");
}

fn token(sub: &str, role: &str) -> String {
    let claims = json!({
        "sub": sub,
        "jti": Uuid::new_v4().to_string(),
        "exp": Utc::now().timestamp() + 600,
        "token_use": "access",
        "uid": 1,
        "email": format!("{sub}@kinetix.test"),
        "role": role,
        "iss": ISSUER,
        "aud": AUDIENCE,
    });
    let mut header = Header::new(Algorithm::RS256);
    header.kid = Some(SIGNING_KEY_ID.to_string());
    let key = EncodingKey::from_rsa_pem(signing_key().pem.as_bytes())
        .expect("the generated key is not a usable RSA key");

    return jsonwebtoken::encode(&header, &claims, &key).expect("the token could not be signed");
}

struct TradingMerchant;
struct NoCatalog;

#[async_trait]
impl MerchantDirectoryPort for TradingMerchant {
    async fn standing_of(
        &self,
        _principal_id: &str,
    ) -> Result<Option<MerchantStanding>, PeerError> {
        return Ok(Some(MerchantStanding {
            merchant_principal_id: MERCHANT.to_string(),
            may_sell: true,
        }));
    }
}

#[async_trait]
impl ProductDirectoryPort for NoCatalog {
    async fn owner_of(&self, _sku: &str) -> Result<Option<String>, PeerError> {
        return Err(PeerError::Unavailable("catalog did not answer".to_string()));
    }
}

#[derive(Clone)]
struct SharedBuffer(Arc<Mutex<Vec<u8>>>);

struct SharedBufferWriter(Arc<Mutex<Vec<u8>>>);

impl<'writer> MakeWriter<'writer> for SharedBuffer {
    type Writer = SharedBufferWriter;

    fn make_writer(&'writer self) -> Self::Writer {
        return SharedBufferWriter(self.0.clone());
    }
}

impl Write for SharedBufferWriter {
    fn write(&mut self, buf: &[u8]) -> IoResult<usize> {
        self.0
            .lock()
            .expect("the log buffer was poisoned")
            .extend_from_slice(buf);
        return Ok(buf.len());
    }

    fn flush(&mut self) -> IoResult<()> {
        return Ok(());
    }
}

struct Audit {
    client: Client,
    metrics: Arc<Metrics>,
    log: Arc<Mutex<Vec<u8>>>,
    _capture: DefaultGuard,
}

impl Audit {
    async fn new(pool: DbPool) -> Audit {
        let log = Arc::new(Mutex::new(Vec::new()));
        let capture =
            tracing::subscriber::set_default(json_logging::subscriber(SharedBuffer(log.clone())));
        let metrics = Arc::new(
            Metrics::new("kinetix-pricing-service", "0.1.0")
                .expect("the metric registry was refused"),
        );

        let rocket = rocket::build()
            .attach(AdminAuditFairing::new(metrics.clone()))
            .manage(pool)
            .manage(verifier())
            .manage(PromotionService::new(
                Arc::new(TradingMerchant),
                Arc::new(NoCatalog),
            ))
            .mount(
                "/",
                routes![
                    create_discount,
                    create_voucher,
                    create_flash_sale,
                    get_voucher,
                    apply_voucher
                ],
            )
            .register(
                "/",
                catchers![not_found, unprocessable, internal_error, other],
            );
        let client = Client::tracked(rocket)
            .await
            .expect("the test rocket did not build");

        return Audit {
            client,
            metrics,
            log,
            _capture: capture,
        };
    }

    fn admin_action_lines(&self) -> Vec<String> {
        let bytes = self
            .log
            .lock()
            .expect("the log buffer was poisoned")
            .clone();
        let text = String::from_utf8(bytes).expect("the log is not UTF-8");

        return text
            .lines()
            .filter(|line| {
                let parsed: Value = serde_json::from_str(line).expect("a log line is not JSON");
                return parsed["event"] == "admin_action";
            })
            .map(str::to_string)
            .collect();
    }

    fn admin_actions(&self) -> Vec<Value> {
        return self
            .admin_action_lines()
            .iter()
            .map(|line| return serde_json::from_str(line).expect("a log line is not JSON"))
            .collect();
    }

    fn counted(&self, action: &str, outcome: &str) -> f64 {
        return counted(&self.metrics, action, outcome);
    }
}

fn counted(metrics: &Metrics, action: &str, outcome: &str) -> f64 {
    let series =
        format!(r#"kinetix_admin_actions_total{{action="{action}",outcome="{outcome}"}} "#);
    let body = metrics.encode().expect("the registry would not encode");
    let line = body
        .lines()
        .find(|line| return line.starts_with(&series))
        .unwrap_or_else(|| panic!("no series {series}on /metrics:\n{body}"));

    return line[series.len()..]
        .parse()
        .expect("the series value is not a number");
}

fn unconnected_pool() -> DbPool {
    return create_pool("postgres://unused:unused@127.0.0.1:1/unused");
}

fn voucher_body(code: &str) -> String {
    return json!({
        "code": code,
        "title": "Ten off",
        "discount_type": "FIXED",
        "value": "10000",
        "min_spend": "0",
        "max_discount": null,
        "quota": 5,
        "expires_at": "2099-01-01T00:00:00Z",
    })
    .to_string();
}

fn discount_body() -> String {
    return json!({
        "title": "Platform week",
        "discount_type": "PERCENTAGE",
        "value": "10",
        "target_product_id": null,
        "target_category_id": null,
        "start_time": "2098-01-01T00:00:00Z",
        "end_time": "2099-01-01T00:00:00Z",
    })
    .to_string();
}

fn bearer(token: &str) -> HttpHeader<'static> {
    return HttpHeader::new("Authorization", format!("Bearer {token}"));
}

fn unique(prefix: &str) -> String {
    return format!("{prefix}-{}", Uuid::new_v4()).to_uppercase();
}

#[test]
fn every_admin_action_is_on_metrics_at_zero_before_any_admin_acts() {
    let metrics = Metrics::new("kinetix-pricing-service", "0.1.0").expect("the registry");

    for action in KNOWN_ADMIN_ACTIONS {
        for outcome in OUTCOMES {
            assert_eq!(
                counted(&metrics, action, outcome),
                0.0,
                "{action} {outcome}"
            );
        }
    }
}

#[test]
fn the_audited_routes_name_the_actions_the_alerts_match() {
    let expected = [
        (routes![create_discount], DISCOUNT_CREATE),
        (routes![create_voucher], VOUCHER_CREATE),
        (routes![create_flash_sale], FLASH_SALE_CREATE),
        (routes![get_voucher], VOUCHER_READ),
        (routes![apply_voucher], VOUCHER_APPLY),
    ];

    for (mounted, action) in expected {
        let name = mounted[0].name.as_deref().expect("a route without a name");
        assert_eq!(admin_action_of_route(name), Some(action), "{name}");
    }

    for create in [DISCOUNT_CREATE, VOUCHER_CREATE, FLASH_SALE_CREATE] {
        assert!(
            create.starts_with("pricing.") && create.ends_with("_create"),
            "{create} escapes the alert's pricing\\..+_create"
        );
    }

    for public in routes![list_discounts, get_flash_sale_for_product] {
        let name = public.name.as_deref().expect("a route without a name");
        assert_eq!(admin_action_of_route(name), None, "{name}");
    }
}

#[tokio::test]
async fn an_admin_create_is_logged_and_counted_with_the_promotion_it_made() {
    let Some(url) = database_url() else {
        return;
    };
    let audit = Audit::new(create_pool(&url)).await;
    let admin = unique("admin");
    let request_id = unique("request");

    let response = audit
        .client
        .post("/api/v1/vouchers")
        .header(bearer(&token(&admin, "admin")))
        .header(HttpHeader::new("Idempotency-Key", unique("key")))
        .header(HttpHeader::new("X-Request-Id", request_id.clone()))
        .body(voucher_body(&unique("AUDIT")))
        .dispatch()
        .await;

    assert_eq!(response.status(), Status::Created);
    let created: Value =
        serde_json::from_str(&response.into_string().await.expect("no body")).expect("not JSON");

    let raw = audit.admin_action_lines();
    assert_eq!(raw.len(), 1, "{raw:?}");
    assert_eq!(
        raw[0].matches(r#""target":"#).count(),
        1,
        "the line names its target twice: {}",
        raw[0]
    );

    let lines = audit.admin_actions();
    let line = &lines[0];
    assert_eq!(line["level"], "INFO");
    assert_eq!(line["message"], "admin_action");
    assert_eq!(line["actor"], admin.as_str());
    assert_eq!(line["action"], VOUCHER_CREATE);
    assert_eq!(line["target"], created["id"], "{line}");
    assert_eq!(line["outcome"], "done");
    assert_eq!(line["request_id"], request_id.as_str());
    assert_eq!(audit.counted(VOUCHER_CREATE, "done"), 1.0);
}

#[tokio::test]
async fn a_sellers_own_create_is_neither_logged_nor_counted() {
    let Some(url) = database_url() else {
        return;
    };
    let audit = Audit::new(create_pool(&url)).await;

    let response = audit
        .client
        .post("/api/v1/vouchers")
        .header(bearer(&token(&unique("seller"), "seller")))
        .header(HttpHeader::new("Idempotency-Key", unique("key")))
        .body(voucher_body(&unique("SELLER")))
        .dispatch()
        .await;

    assert_eq!(response.status(), Status::Created);
    assert!(
        audit.admin_actions().is_empty(),
        "{:?}",
        audit.admin_actions()
    );
    for outcome in OUTCOMES {
        assert_eq!(audit.counted(VOUCHER_CREATE, outcome), 0.0, "{outcome}");
    }
}

#[tokio::test]
async fn an_admin_create_a_guard_refuses_is_counted_as_refused_with_no_target() {
    let audit = Audit::new(unconnected_pool()).await;
    let admin = unique("admin");

    let response = audit
        .client
        .post("/api/v1/discounts")
        .header(bearer(&token(&admin, "admin")))
        .body(discount_body())
        .dispatch()
        .await;

    assert_eq!(response.status(), Status::BadRequest);
    let lines = audit.admin_actions();
    assert_eq!(lines.len(), 1, "{lines:?}");
    assert_eq!(lines[0]["action"], DISCOUNT_CREATE);
    assert_eq!(lines[0]["actor"], admin.as_str());
    assert_eq!(lines[0]["target"], "-");
    assert_eq!(lines[0]["outcome"], "refused");
    assert_eq!(lines[0]["request_id"], "-");
    assert_eq!(audit.counted(DISCOUNT_CREATE, "refused"), 1.0);
}

#[tokio::test]
async fn an_admin_create_while_the_database_is_down_is_counted_as_failed() {
    let audit = Audit::new(unconnected_pool()).await;

    let response = audit
        .client
        .post("/api/v1/discounts")
        .header(bearer(&token(&unique("admin"), "admin")))
        .header(HttpHeader::new("Idempotency-Key", unique("key")))
        .body(discount_body())
        .dispatch()
        .await;

    assert_eq!(response.status(), Status::ServiceUnavailable);
    let lines = audit.admin_actions();
    assert_eq!(lines.len(), 1, "{lines:?}");
    assert_eq!(lines[0]["outcome"], "failed");
    assert_eq!(audit.counted(DISCOUNT_CREATE, "failed"), 1.0);
    assert_eq!(audit.counted(DISCOUNT_CREATE, "done"), 0.0);
}

#[tokio::test]
async fn an_admin_read_of_a_vouchers_quota_is_audited_under_its_code() {
    let Some(url) = database_url() else {
        return;
    };
    let audit = Audit::new(create_pool(&url)).await;
    let admin_token = token(&unique("admin"), "admin");
    let code = unique("QUOTA");
    let missing = unique("MISSING");

    let created = audit
        .client
        .post("/api/v1/vouchers")
        .header(bearer(&admin_token))
        .header(HttpHeader::new("Idempotency-Key", unique("key")))
        .body(voucher_body(&code))
        .dispatch()
        .await;
    assert_eq!(created.status(), Status::Created);

    let read = audit
        .client
        .get(format!("/api/v1/vouchers/{}", code.to_lowercase()))
        .header(bearer(&admin_token))
        .dispatch()
        .await;
    assert_eq!(read.status(), Status::Ok);

    let unknown = audit
        .client
        .get(format!("/api/v1/vouchers/{missing}"))
        .header(bearer(&admin_token))
        .dispatch()
        .await;
    assert_eq!(unknown.status(), Status::NotFound);

    let applied = audit
        .client
        .post("/api/v1/vouchers/apply")
        .header(bearer(&admin_token))
        .body(json!({"code": code, "cart_subtotal": "50000"}).to_string())
        .dispatch()
        .await;
    assert_eq!(applied.status(), Status::Ok);

    let anonymous = audit
        .client
        .get(format!("/api/v1/vouchers/{code}"))
        .dispatch()
        .await;
    assert_eq!(anonymous.status(), Status::Ok);

    let seen: Vec<(String, String, String)> = audit
        .admin_actions()
        .iter()
        .map(|line| {
            return (
                line["action"].as_str().unwrap_or("").to_string(),
                line["target"].as_str().unwrap_or("").to_string(),
                line["outcome"].as_str().unwrap_or("").to_string(),
            );
        })
        .filter(|(action, _, _)| return action != VOUCHER_CREATE)
        .collect();
    assert_eq!(
        seen,
        vec![
            (VOUCHER_READ.to_string(), code.clone(), "done".to_string()),
            (VOUCHER_READ.to_string(), missing, "refused".to_string()),
            (VOUCHER_APPLY.to_string(), code, "done".to_string()),
        ]
    );
    assert_eq!(audit.counted(VOUCHER_READ, "done"), 1.0);
    assert_eq!(audit.counted(VOUCHER_READ, "refused"), 1.0);
    assert_eq!(audit.counted(VOUCHER_APPLY, "done"), 1.0);
}
