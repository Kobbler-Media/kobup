use kobup_service::{AppState, Config, auth, db, rate_limit::RateLimiter, storage::LocalStorage};
use reqwest::{Client, StatusCode};
use serde_json::{Value, json};
use std::io::Write;
use std::sync::Arc;

/// Build a minimal valid Android binary XML representing an AndroidManifest.xml
/// with the given package name, version code, and version name.
///
/// The binary format follows the Android AXML structure that `axmldecoder` expects:
///   - File header (type=XML, header_size=8, size=total)
///   - StringPool chunk (UTF-8 flag set)
///   - ResourceMap chunk (maps attribute indices to Android resource IDs)
///   - XmlStartNameSpace node
///   - XmlStartElement node (<manifest> with package, versionCode, versionName)
///   - XmlEndElement node
///   - XmlEndNameSpace node
fn build_binary_manifest(package: &str, version_code: u32, version_name: &str) -> Vec<u8> {
    // Strings in order: index 0..6
    // 0: "http://schemas.android.com/apk/res/android"  (namespace URI)
    // 1: "android"  (namespace prefix)
    // 2: "manifest" (element tag)
    // 3: "package"  (attribute name)
    // 4: package    (attribute value - the actual package name)
    // 5: "versionCode" (attribute name)
    // 6: "versionName" (attribute name)
    // 7: version_name  (attribute value)
    let strings: Vec<&str> = vec![
        "http://schemas.android.com/apk/res/android",
        "android",
        "manifest",
        "package",
        package,
        "versionCode",
        "versionName",
        version_name,
    ];

    // Build UTF-8 string pool data
    let mut string_offsets: Vec<u32> = Vec::new();
    let mut string_data: Vec<u8> = Vec::new();
    for s in &strings {
        string_offsets.push(string_data.len() as u32);
        let bytes = s.as_bytes();
        // UTF-8 string format: u16 char_count, u8 byte_count, bytes, null
        // Actually for axmldecoder's UTF-8 parsing: offset+0 = char_len (unused by parser),
        // offset+1 = byte_len, offset+2.. = bytes
        string_data.push(bytes.len() as u8); // charLen (simplified)
        string_data.push(bytes.len() as u8); // byteLen
        string_data.extend_from_slice(bytes);
        string_data.push(0); // null terminator
    }

    // Offsets array
    let mut offsets_bytes: Vec<u8> = Vec::new();
    for off in &string_offsets {
        offsets_bytes.extend_from_slice(&off.to_le_bytes());
    }

    let string_count = strings.len() as u32;
    // StringPool header is 28 bytes (ChunkHeader=8 + 5*u32=20)
    let sp_header_size: u16 = 28;
    let string_start = sp_header_size as u32 + (string_count * 4); // after header + offsets
    let sp_total_size = string_start + string_data.len() as u32;

    let mut string_pool_chunk: Vec<u8> = Vec::new();
    // ChunkHeader: type=0x0001 (StringPool), header_size=28, size=sp_total_size
    string_pool_chunk.extend_from_slice(&0x0001u16.to_le_bytes()); // type
    string_pool_chunk.extend_from_slice(&sp_header_size.to_le_bytes()); // header_size
    string_pool_chunk.extend_from_slice(&sp_total_size.to_le_bytes()); // size
    // StringPoolHeader extra fields
    string_pool_chunk.extend_from_slice(&string_count.to_le_bytes()); // string_count
    string_pool_chunk.extend_from_slice(&0u32.to_le_bytes()); // style_count
    string_pool_chunk.extend_from_slice(&(1u32 << 8).to_le_bytes()); // flags (UTF-8)
    string_pool_chunk.extend_from_slice(&string_start.to_le_bytes()); // string_start
    string_pool_chunk.extend_from_slice(&0u32.to_le_bytes()); // style_start
    // String offsets
    string_pool_chunk.extend_from_slice(&offsets_bytes);
    // String data
    string_pool_chunk.extend_from_slice(&string_data);

    // ResourceMap chunk - maps attribute name string indices to Android resource IDs
    // We have attributes at indices: 3=package, 5=versionCode, 6=versionName
    // The resource map must cover indices 0..max_attr_index+1
    // Resource IDs: 0=none, 1=none, 2=none, 3=package(0x0), 5=versionCode(0x0101021b), 6=versionName(0x0101021c)
    let resource_ids: Vec<u32> = vec![
        0,          // 0: namespace URI (not an attr)
        0,          // 1: namespace prefix (not an attr)
        0,          // 2: element tag (not an attr)
        0,          // 3: "package" - no resource ID needed
        0,          // 4: package value (not an attr name)
        0x0101021b, // 5: "versionCode"
        0x0101021c, // 6: "versionName"
        0,          // 7: version_name value (not an attr name)
    ];
    let rm_header_size: u16 = 8;
    let rm_size = rm_header_size as u32 + (resource_ids.len() as u32 * 4);
    let mut resource_map_chunk: Vec<u8> = Vec::new();
    resource_map_chunk.extend_from_slice(&0x0180u16.to_le_bytes()); // type=XmlResourceMap
    resource_map_chunk.extend_from_slice(&rm_header_size.to_le_bytes());
    resource_map_chunk.extend_from_slice(&rm_size.to_le_bytes());
    for id in &resource_ids {
        resource_map_chunk.extend_from_slice(&id.to_le_bytes());
    }

    // XmlNode helper: each node has XmlNodeHeader (ChunkHeader(8) + line_no(4) + comment(4) = 16)
    let node_header_size: u16 = 16;

    // XmlStartNameSpace: header(16) + prefix(4) + uri(4) = 24
    let mut ns_start: Vec<u8> = Vec::new();
    let ns_start_size: u32 = 24;
    ns_start.extend_from_slice(&0x0100u16.to_le_bytes()); // type=XmlStartNameSpace
    ns_start.extend_from_slice(&node_header_size.to_le_bytes());
    ns_start.extend_from_slice(&ns_start_size.to_le_bytes());
    ns_start.extend_from_slice(&1u32.to_le_bytes()); // line_no
    ns_start.extend_from_slice(&0xFFFFFFFFu32.to_le_bytes()); // comment (-1)
    ns_start.extend_from_slice(&1u32.to_le_bytes()); // prefix = string index 1 ("android")
    ns_start.extend_from_slice(&0u32.to_le_bytes()); // uri = string index 0

    // XmlStartElement: header(16) + XmlAttrExt(20) + attributes(N*20)
    // Attributes: package (string), versionCode (dec), versionName (string)
    let attr_count: u16 = 3;
    let attr_size: u16 = 20; // each attribute is 20 bytes
    let elem_start_size: u32 = 16 + 20 + (attr_count as u32 * attr_size as u32);
    let mut elem_start: Vec<u8> = Vec::new();
    elem_start.extend_from_slice(&0x0102u16.to_le_bytes()); // type=XmlStartElement
    elem_start.extend_from_slice(&node_header_size.to_le_bytes());
    elem_start.extend_from_slice(&elem_start_size.to_le_bytes());
    elem_start.extend_from_slice(&1u32.to_le_bytes()); // line_no
    elem_start.extend_from_slice(&0xFFFFFFFFu32.to_le_bytes()); // comment
    // XmlAttrExt
    elem_start.extend_from_slice(&0xFFFFFFFFu32.to_le_bytes()); // ns (-1 = no namespace for element)
    elem_start.extend_from_slice(&2u32.to_le_bytes()); // name = string index 2 ("manifest")
    elem_start.extend_from_slice(&0u16.to_le_bytes()); // attribute_start (offset from start of attrs)
    elem_start.extend_from_slice(&attr_size.to_le_bytes()); // attribute_size
    elem_start.extend_from_slice(&attr_count.to_le_bytes()); // attribute_count
    elem_start.extend_from_slice(&0u16.to_le_bytes()); // id_index
    elem_start.extend_from_slice(&0u16.to_le_bytes()); // class_index
    elem_start.extend_from_slice(&0u16.to_le_bytes()); // style_index

    // Attribute: package (ns=none, name=3, raw_value=4, typed=string pointing to 4)
    elem_start.extend_from_slice(&0xFFFFFFFFu32.to_le_bytes()); // ns (-1 = no namespace)
    elem_start.extend_from_slice(&3u32.to_le_bytes()); // name = "package"
    elem_start.extend_from_slice(&4u32.to_le_bytes()); // raw_value = string index 4
    // ResourceValue: size=8, res=0, data_type=String(0x03), data=4
    elem_start.extend_from_slice(&8u16.to_le_bytes());
    elem_start.push(0); // res
    elem_start.push(0x03); // data_type = String
    elem_start.extend_from_slice(&4u32.to_le_bytes()); // data = string index 4

    // Attribute: versionCode (ns=android, name=5, typed=Dec)
    elem_start.extend_from_slice(&0u32.to_le_bytes()); // ns = string index 0 (android URI)
    elem_start.extend_from_slice(&5u32.to_le_bytes()); // name = "versionCode"
    elem_start.extend_from_slice(&0xFFFFFFFFu32.to_le_bytes()); // raw_value (-1)
    elem_start.extend_from_slice(&8u16.to_le_bytes());
    elem_start.push(0);
    elem_start.push(0x10); // data_type = Dec
    elem_start.extend_from_slice(&version_code.to_le_bytes());

    // Attribute: versionName (ns=android, name=6, typed=String pointing to 7)
    elem_start.extend_from_slice(&0u32.to_le_bytes()); // ns = string index 0
    elem_start.extend_from_slice(&6u32.to_le_bytes()); // name = "versionName"
    elem_start.extend_from_slice(&7u32.to_le_bytes()); // raw_value = string index 7
    elem_start.extend_from_slice(&8u16.to_le_bytes());
    elem_start.push(0);
    elem_start.push(0x03); // data_type = String
    elem_start.extend_from_slice(&7u32.to_le_bytes()); // data = string index 7

    // XmlEndElement: header(16) + ns(4) + name(4) = 24
    let mut elem_end: Vec<u8> = Vec::new();
    let elem_end_size: u32 = 24;
    elem_end.extend_from_slice(&0x0103u16.to_le_bytes()); // type=XmlEndElement
    elem_end.extend_from_slice(&node_header_size.to_le_bytes());
    elem_end.extend_from_slice(&elem_end_size.to_le_bytes());
    elem_end.extend_from_slice(&1u32.to_le_bytes()); // line_no
    elem_end.extend_from_slice(&0xFFFFFFFFu32.to_le_bytes()); // comment
    elem_end.extend_from_slice(&0xFFFFFFFFu32.to_le_bytes()); // ns
    elem_end.extend_from_slice(&2u32.to_le_bytes()); // name = "manifest"

    // XmlEndNameSpace: header(16) + prefix(4) + uri(4) = 24
    let mut ns_end: Vec<u8> = Vec::new();
    let ns_end_size: u32 = 24;
    ns_end.extend_from_slice(&0x0101u16.to_le_bytes()); // type=XmlEndNameSpace
    ns_end.extend_from_slice(&node_header_size.to_le_bytes());
    ns_end.extend_from_slice(&ns_end_size.to_le_bytes());
    ns_end.extend_from_slice(&1u32.to_le_bytes()); // line_no
    ns_end.extend_from_slice(&0xFFFFFFFFu32.to_le_bytes()); // comment
    ns_end.extend_from_slice(&1u32.to_le_bytes()); // prefix
    ns_end.extend_from_slice(&0u32.to_le_bytes()); // uri

    // Assemble the full document
    let body_size = string_pool_chunk.len()
        + resource_map_chunk.len()
        + ns_start.len()
        + elem_start.len()
        + elem_end.len()
        + ns_end.len();
    let file_header_size: u16 = 8;
    let total_size: u32 = file_header_size as u32 + body_size as u32;

    let mut out: Vec<u8> = Vec::new();
    // File header: type=0x0003 (Xml), header_size=8, size=total
    out.extend_from_slice(&0x0003u16.to_le_bytes());
    out.extend_from_slice(&file_header_size.to_le_bytes());
    out.extend_from_slice(&total_size.to_le_bytes());
    out.extend_from_slice(&string_pool_chunk);
    out.extend_from_slice(&resource_map_chunk);
    out.extend_from_slice(&ns_start);
    out.extend_from_slice(&elem_start);
    out.extend_from_slice(&elem_end);
    out.extend_from_slice(&ns_end);
    out
}

/// Create a minimal valid APK (ZIP containing a binary AndroidManifest.xml).
fn create_test_apk(package: &str, version_code: u32, version_name: &str) -> Vec<u8> {
    let manifest = build_binary_manifest(package, version_code, version_name);
    let buf = Vec::new();
    let mut zip = zip::ZipWriter::new(std::io::Cursor::new(buf));
    zip.start_file::<_, ()>(
        "AndroidManifest.xml",
        zip::write::SimpleFileOptions::default(),
    )
    .unwrap();
    zip.write_all(&manifest).unwrap();
    zip.start_file::<_, ()>("classes.dex", zip::write::SimpleFileOptions::default())
        .unwrap();
    zip.write_all(b"fake dex").unwrap();
    zip.finish().unwrap().into_inner()
}

/// Helper to create a project and return its ID.
async fn create_project(
    client: &Client,
    base_url: &str,
    token: &str,
    slug: &str,
    name: &str,
) -> Value {
    let resp = client
        .post(format!("{base_url}/api/v1/admin/projects"))
        .bearer_auth(token)
        .json(&json!({"slug": slug, "name": name}))
        .send()
        .await
        .expect("create project");
    assert!(
        resp.status() == StatusCode::CREATED || resp.status() == StatusCode::OK,
        "create project returned {}",
        resp.status()
    );
    resp.json().await.expect("json")
}

/// Helper to upload an APK to a project and return the upload response.
async fn upload_apk(
    client: &Client,
    base_url: &str,
    token: &str,
    project_id: &str,
    apk_bytes: Vec<u8>,
) -> Value {
    let part = reqwest::multipart::Part::bytes(apk_bytes)
        .file_name("test.apk")
        .mime_str("application/vnd.android.package-archive")
        .unwrap();
    let form = reqwest::multipart::Form::new().part("apk", part);

    let resp = client
        .post(format!(
            "{base_url}/api/v1/admin/projects/{project_id}/upload"
        ))
        .bearer_auth(token)
        .multipart(form)
        .send()
        .await
        .expect("upload");
    assert_eq!(resp.status(), StatusCode::CREATED, "upload failed");
    resp.json().await.expect("json")
}

async fn setup() -> (Client, String, String) {
    let (client, base_url, token, _pool, _storage_root) = setup_with_pool().await;
    (client, base_url, token)
}

async fn setup_with_pool() -> (
    Client,
    String,
    String,
    kobup_service::DbPool,
    std::path::PathBuf,
) {
    setup_with(|_| {}).await
}

async fn setup_with(
    configure: impl FnOnce(&mut Config),
) -> (
    Client,
    String,
    String,
    kobup_service::DbPool,
    std::path::PathBuf,
) {
    let _ = rustls::crypto::ring::default_provider().install_default();

    let pool = db::connect("sqlite::memory:").await.expect("connect");
    db::migrate(&pool).await.expect("migrate");

    let password = "test-password-123";
    let hash = auth::hash_password(password).expect("hash");
    let user_id = uuid::Uuid::now_v7().to_string();
    sqlx::query("INSERT INTO users (id, email, password_hash, role) VALUES (?, ?, ?, 'admin')")
        .bind(&user_id)
        .bind("admin@test.local")
        .bind(&hash)
        .execute(&pool)
        .await
        .expect("insert user");

    let tmp = tempfile::tempdir().expect("tempdir");
    let storage_root = tmp.path().to_path_buf();
    let storage = LocalStorage::new(tmp.path());
    storage.ensure_dirs().await.expect("ensure dirs");

    let mut config = Config {
        database_url: "sqlite::memory:".to_string(),
        bind_addr: "127.0.0.1:0".to_string(),
        base_url: "http://localhost".to_string(),
        storage_root: tmp.path().to_string_lossy().to_string(),
        admin_email: "admin@test.local".to_string(),
        admin_bootstrap_password: Some(password.to_string()),
        web_origins: vec![],
        http_max_body_bytes: 500 * 1024 * 1024,
        token_length: 32,
        fcm_service_account_json: None,
        github_token: None,
        max_releases_per_channel: 10,
        github_poll_interval_secs: 3600,
        trusted_proxy_hops: 1,
        pin_service_url: "http://127.0.0.1:9".to_string(),
    };
    configure(&mut config);

    let pool_handle = pool.clone();
    let state = AppState {
        pool,
        config: Arc::new(config),
        storage: Arc::new(storage),
        fcm: None,
        login_limiter: Arc::new(RateLimiter::new(5, std::time::Duration::from_secs(60))),
    };

    let app = kobup_service::create_router(state);
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0")
        .await
        .expect("bind");
    let addr = listener.local_addr().expect("local_addr");
    let base_url = format!("http://{addr}");

    tokio::spawn(async move {
        axum::serve(
            listener,
            app.into_make_service_with_connect_info::<std::net::SocketAddr>(),
        )
        .await
        .ok();
    });

    let client = Client::builder()
        .redirect(reqwest::redirect::Policy::none())
        .build()
        .expect("client");

    // Login to get admin token
    let resp = client
        .post(format!("{base_url}/api/v1/admin/login"))
        .json(&json!({"email": "admin@test.local", "password": password}))
        .send()
        .await
        .expect("login");
    assert_eq!(resp.status(), StatusCode::OK);
    let body: Value = resp.json().await.expect("json");
    let token = body["token"].as_str().expect("token").to_string();

    // Keep tempdir alive by leaking it (tests are short-lived)
    std::mem::forget(tmp);

    (client, base_url, token, pool_handle, storage_root)
}

#[tokio::test]
async fn test_health() {
    let (client, base_url, _) = setup().await;

    let resp = client
        .get(format!("{base_url}/health"))
        .send()
        .await
        .expect("health");
    assert_eq!(resp.status(), StatusCode::OK);
    assert_eq!(resp.text().await.unwrap(), "ok");
}

#[tokio::test]
async fn test_login_success() {
    let (client, base_url, _) = setup().await;

    let resp = client
        .post(format!("{base_url}/api/v1/admin/login"))
        .json(&json!({"email": "admin@test.local", "password": "test-password-123"}))
        .send()
        .await
        .expect("login");
    assert_eq!(resp.status(), StatusCode::OK);

    let body: Value = resp.json().await.expect("json");
    assert!(body["token"].is_string());
    assert_eq!(body["role"], "admin");
    assert_eq!(body["email"], "admin@test.local");
}

#[tokio::test]
async fn test_login_failure() {
    let (client, base_url, _) = setup().await;

    let resp = client
        .post(format!("{base_url}/api/v1/admin/login"))
        .json(&json!({"email": "admin@test.local", "password": "wrong"}))
        .send()
        .await
        .expect("login");
    assert_eq!(resp.status(), StatusCode::UNAUTHORIZED);
}

#[tokio::test]
async fn test_unauthorized_access() {
    let (client, base_url, _) = setup().await;

    let resp = client
        .get(format!("{base_url}/api/v1/admin/projects"))
        .send()
        .await
        .expect("projects");
    assert_eq!(resp.status(), StatusCode::UNAUTHORIZED);
}

#[tokio::test]
async fn test_create_project() {
    let (client, base_url, token) = setup().await;

    let resp = client
        .post(format!("{base_url}/api/v1/admin/projects"))
        .bearer_auth(&token)
        .json(&json!({
            "slug": "test-app",
            "name": "Test App",
            "packageName": "com.example.test",
            "description": "A test project"
        }))
        .send()
        .await
        .expect("create project");
    assert_eq!(resp.status(), StatusCode::CREATED);

    let body: Value = resp.json().await.expect("json");
    assert_eq!(body["slug"], "test-app");
    assert_eq!(body["name"], "Test App");
    assert_eq!(body["packageName"], "com.example.test");

    // Verify auto-created stable channel
    let channels = body["channels"].as_array().expect("channels array");
    assert_eq!(channels.len(), 1);
    assert_eq!(channels[0]["name"], "stable");
    assert!(channels[0]["isDefault"].as_bool().unwrap());
}

#[tokio::test]
async fn test_create_project_idempotent() {
    let (client, base_url, token) = setup().await;

    let payload = json!({
        "slug": "idempotent-app",
        "name": "Idempotent App",
    });

    let resp1 = client
        .post(format!("{base_url}/api/v1/admin/projects"))
        .bearer_auth(&token)
        .json(&payload)
        .send()
        .await
        .expect("create 1");
    assert_eq!(resp1.status(), StatusCode::CREATED);
    let body1: Value = resp1.json().await.expect("json");

    let resp2 = client
        .post(format!("{base_url}/api/v1/admin/projects"))
        .bearer_auth(&token)
        .json(&payload)
        .send()
        .await
        .expect("create 2");
    assert_eq!(resp2.status(), StatusCode::OK);
    let body2: Value = resp2.json().await.expect("json");

    assert_eq!(body1["id"], body2["id"]);
}

#[tokio::test]
async fn test_list_projects() {
    let (client, base_url, token) = setup().await;

    // Create two projects
    for slug in &["list-a", "list-b"] {
        client
            .post(format!("{base_url}/api/v1/admin/projects"))
            .bearer_auth(&token)
            .json(&json!({"slug": slug, "name": slug}))
            .send()
            .await
            .expect("create");
    }

    let resp = client
        .get(format!("{base_url}/api/v1/admin/projects"))
        .bearer_auth(&token)
        .send()
        .await
        .expect("list");
    assert_eq!(resp.status(), StatusCode::OK);

    let body: Value = resp.json().await.expect("json");
    let projects = body.as_array().expect("array");
    assert!(projects.len() >= 2);
}

#[tokio::test]
async fn test_catalog() {
    let (client, base_url, token) = setup().await;

    // Create a project (visibility defaults to "public")
    client
        .post(format!("{base_url}/api/v1/admin/projects"))
        .bearer_auth(&token)
        .json(&json!({"slug": "catalog-app", "name": "Catalog App"}))
        .send()
        .await
        .expect("create");

    // Catalog is a public endpoint, no auth needed
    let resp = client
        .get(format!("{base_url}/api/v1/catalog"))
        .send()
        .await
        .expect("catalog");
    assert_eq!(resp.status(), StatusCode::OK);

    let body: Value = resp.json().await.expect("json");
    let items = body.as_array().expect("array");
    assert!(items.iter().any(|i| i["slug"] == "catalog-app"));
}

/// Upload an APK to a specific channel and publish it.
async fn upload_published(
    client: &Client,
    base_url: &str,
    token: &str,
    project_id: &str,
    channel: &str,
    apk_bytes: Vec<u8>,
) {
    let part = reqwest::multipart::Part::bytes(apk_bytes)
        .file_name("test.apk")
        .mime_str("application/vnd.android.package-archive")
        .unwrap();
    let form = reqwest::multipart::Form::new()
        .part("apk", part)
        .text("channel", channel.to_string())
        .text("publish", "true");
    let resp = client
        .post(format!(
            "{base_url}/api/v1/admin/projects/{project_id}/upload"
        ))
        .bearer_auth(token)
        .multipart(form)
        .send()
        .await
        .expect("upload");
    assert_eq!(resp.status(), StatusCode::CREATED, "publish upload failed");
}

/// Fetch the catalog (optionally filtered by channel) and return the item for `slug`.
async fn catalog_item(client: &Client, base_url: &str, channel: Option<&str>, slug: &str) -> Value {
    let url = match channel {
        Some(c) => format!("{base_url}/api/v1/catalog?channel={c}"),
        None => format!("{base_url}/api/v1/catalog"),
    };
    let resp = client.get(url).send().await.expect("catalog");
    assert_eq!(resp.status(), StatusCode::OK);
    let body: Value = resp.json().await.expect("json");
    body.as_array()
        .expect("array")
        .iter()
        .find(|i| i["slug"] == slug)
        .cloned()
        .expect("catalog item present")
}

#[tokio::test]
async fn test_catalog_channel_filter() {
    let (client, base_url, token) = setup().await;
    let project = create_project(&client, &base_url, &token, "chan-catalog", "Chan Catalog").await;
    let project_id = project["id"].as_str().unwrap();

    // Add a "beta" channel alongside the auto-created "stable".
    let resp = client
        .post(format!(
            "{base_url}/api/v1/admin/projects/{project_id}/channels"
        ))
        .bearer_auth(&token)
        .json(&json!({"name": "beta"}))
        .send()
        .await
        .expect("create beta");
    assert_eq!(resp.status(), StatusCode::CREATED);

    // Publish a lower stable release and a higher beta release.
    upload_published(
        &client,
        &base_url,
        &token,
        project_id,
        "stable",
        create_test_apk("com.test.chan", 1, "1.0.0"),
    )
    .await;
    upload_published(
        &client,
        &base_url,
        &token,
        project_id,
        "beta",
        create_test_apk("com.test.chan", 2, "2.0.0-beta"),
    )
    .await;

    // No channel: newest across all channels wins (beta, vc 2).
    let item = catalog_item(&client, &base_url, None, "chan-catalog").await;
    assert_eq!(item["latestVersion"]["versionName"], "2.0.0-beta");
    assert_eq!(item["latestVersion"]["channel"], "beta");

    // channel=stable: only the stable release, even though beta is newer.
    let item = catalog_item(&client, &base_url, Some("stable"), "chan-catalog").await;
    assert_eq!(item["latestVersion"]["versionName"], "1.0.0");
    assert_eq!(item["latestVersion"]["channel"], "stable");

    // channel=beta: the beta release.
    let item = catalog_item(&client, &base_url, Some("beta"), "chan-catalog").await;
    assert_eq!(item["latestVersion"]["versionName"], "2.0.0-beta");
    assert_eq!(item["latestVersion"]["channel"], "beta");
}

// ---------------------------------------------------------------------------
// Upload + Download flow
// ---------------------------------------------------------------------------

#[tokio::test]
async fn test_upload_invalid_apk() {
    let (client, base_url, token) = setup().await;
    let project = create_project(&client, &base_url, &token, "bad-apk", "Bad APK").await;
    let project_id = project["id"].as_str().unwrap();

    // Upload a non-APK file (not a valid ZIP)
    let part = reqwest::multipart::Part::bytes(b"not a zip".to_vec())
        .file_name("garbage.apk")
        .mime_str("application/vnd.android.package-archive")
        .unwrap();
    let form = reqwest::multipart::Form::new().part("apk", part);

    let resp = client
        .post(format!(
            "{base_url}/api/v1/admin/projects/{project_id}/upload"
        ))
        .bearer_auth(&token)
        .multipart(form)
        .send()
        .await
        .expect("upload");
    assert_eq!(resp.status(), StatusCode::BAD_REQUEST);
}

#[tokio::test]
async fn test_upload_rejects_path_traversal_filename() {
    let (client, base_url, token) = setup().await;
    let project = create_project(&client, &base_url, &token, "traversal", "Traversal").await;
    let project_id = project["id"].as_str().unwrap();

    let apk_bytes = create_test_apk("com.test.traversal", 1, "1.0.0");
    let part = reqwest::multipart::Part::bytes(apk_bytes)
        .file_name("../../x.apk")
        .mime_str("application/vnd.android.package-archive")
        .unwrap();
    let form = reqwest::multipart::Form::new().part("apk", part);

    let resp = client
        .post(format!(
            "{base_url}/api/v1/admin/projects/{project_id}/upload"
        ))
        .bearer_auth(&token)
        .multipart(form)
        .send()
        .await
        .expect("upload");
    assert_eq!(resp.status(), StatusCode::BAD_REQUEST);

    // No release/artifact record should have been created either.
    let resp = client
        .get(format!(
            "{base_url}/api/v1/admin/projects/{project_id}/releases"
        ))
        .bearer_auth(&token)
        .send()
        .await
        .expect("list releases");
    let releases: Value = resp.json().await.unwrap();
    assert_eq!(releases.as_array().unwrap().len(), 0);
}

#[tokio::test]
async fn test_upload_and_download_apk() {
    let (client, base_url, token) = setup().await;
    let project = create_project(&client, &base_url, &token, "upload-dl", "Upload DL").await;
    let project_id = project["id"].as_str().unwrap();

    let apk_bytes = create_test_apk("com.test.upload", 1, "1.0.0");
    let upload_resp = upload_apk(&client, &base_url, &token, project_id, apk_bytes.clone()).await;

    // Verify response structure
    assert_eq!(upload_resp["release"]["versionCode"], 1);
    assert_eq!(upload_resp["release"]["versionName"], "1.0.0");
    assert_eq!(upload_resp["artifact"]["packageName"], "com.test.upload");
    assert!(upload_resp["artifact"]["id"].is_string());
    assert!(upload_resp["artifact"]["sha256"].is_string());
    assert!(upload_resp["artifact"]["sizeBytes"].as_i64().unwrap() > 0);

    // List releases and verify the new release appears
    let resp = client
        .get(format!(
            "{base_url}/api/v1/admin/projects/{project_id}/releases"
        ))
        .bearer_auth(&token)
        .send()
        .await
        .expect("list releases");
    assert_eq!(resp.status(), StatusCode::OK);
    let releases: Value = resp.json().await.unwrap();
    let releases = releases.as_array().unwrap();
    assert_eq!(releases.len(), 1);
    assert_eq!(releases[0]["versionCode"], 1);

    // Download the artifact
    let artifact_id = upload_resp["artifact"]["id"].as_str().unwrap();
    let resp = client
        .get(format!(
            "{base_url}/api/v1/artifacts/{artifact_id}/download"
        ))
        .send()
        .await
        .expect("download");
    assert_eq!(resp.status(), StatusCode::OK);
    let downloaded = resp.bytes().await.unwrap();
    assert_eq!(downloaded.as_ref(), apk_bytes.as_slice());
}

#[tokio::test]
async fn test_upload_duplicate_version_code() {
    let (client, base_url, token) = setup().await;
    let project = create_project(&client, &base_url, &token, "dup-vc", "Dup VC").await;
    let project_id = project["id"].as_str().unwrap();

    let apk = create_test_apk("com.test.dup", 42, "1.0.0");
    upload_apk(&client, &base_url, &token, project_id, apk).await;

    // Second upload with same version_code should conflict
    let apk2 = create_test_apk("com.test.dup", 42, "1.0.1");
    let part = reqwest::multipart::Part::bytes(apk2)
        .file_name("test.apk")
        .mime_str("application/vnd.android.package-archive")
        .unwrap();
    let form = reqwest::multipart::Form::new().part("apk", part);
    let resp = client
        .post(format!(
            "{base_url}/api/v1/admin/projects/{project_id}/upload"
        ))
        .bearer_auth(&token)
        .multipart(form)
        .send()
        .await
        .expect("upload 2");
    assert_eq!(resp.status(), StatusCode::CONFLICT);
}

// ---------------------------------------------------------------------------
// Release publish flow
// ---------------------------------------------------------------------------

#[tokio::test]
async fn test_publish_release() {
    let (client, base_url, token) = setup().await;
    let project = create_project(&client, &base_url, &token, "pub-app", "Pub App").await;
    let project_id = project["id"].as_str().unwrap();

    let apk = create_test_apk("com.test.pub", 1, "1.0.0");
    let upload_resp = upload_apk(&client, &base_url, &token, project_id, apk).await;
    let release_id = upload_resp["release"]["id"].as_str().unwrap();

    // Verify release is NOT published
    assert!(upload_resp["release"]["publishedAt"].is_null());

    // Publish the release
    let resp = client
        .post(format!(
            "{base_url}/api/v1/admin/releases/{release_id}/publish"
        ))
        .bearer_auth(&token)
        .send()
        .await
        .expect("publish");
    assert_eq!(resp.status(), StatusCode::OK);
    let body: Value = resp.json().await.unwrap();
    assert!(body["publishedAt"].is_string());

    // Publishing again should return CONFLICT
    let resp = client
        .post(format!(
            "{base_url}/api/v1/admin/releases/{release_id}/publish"
        ))
        .bearer_auth(&token)
        .send()
        .await
        .expect("publish again");
    assert_eq!(resp.status(), StatusCode::CONFLICT);

    // Verify catalog now shows version info
    let resp = client
        .get(format!("{base_url}/api/v1/catalog"))
        .send()
        .await
        .expect("catalog");
    assert_eq!(resp.status(), StatusCode::OK);
    let catalog: Value = resp.json().await.unwrap();
    let items = catalog.as_array().unwrap();
    let item = items.iter().find(|i| i["slug"] == "pub-app").unwrap();
    assert_eq!(item["latestVersion"]["versionCode"], 1);
    assert_eq!(item["latestVersion"]["versionName"], "1.0.0");
}

// ---------------------------------------------------------------------------
// Device registration + heartbeat
// ---------------------------------------------------------------------------

#[tokio::test]
async fn test_device_registration_and_heartbeat() {
    let (client, base_url, token) = setup().await;

    // Register a device
    let resp = client
        .post(format!("{base_url}/api/v1/devices/register"))
        .json(&json!({
            "deviceName": "Test Device",
            "sdkInt": 33,
            "abis": ["arm64-v8a", "armeabi-v7a"],
            "isTv": false,
            "kobupVersion": "0.1.0"
        }))
        .send()
        .await
        .expect("register");
    assert_eq!(resp.status(), StatusCode::CREATED);
    let body: Value = resp.json().await.unwrap();
    let device_id = body["deviceId"].as_str().unwrap().to_string();

    // Heartbeat
    let resp = client
        .post(format!("{base_url}/api/v1/devices/{device_id}/heartbeat"))
        .json(&json!({"kobupVersion": "0.1.1"}))
        .send()
        .await
        .expect("heartbeat");
    assert_eq!(resp.status(), StatusCode::NO_CONTENT);

    // Push token
    let resp = client
        .post(format!("{base_url}/api/v1/devices/{device_id}/push-token"))
        .json(&json!({"token": "fake-fcm-token-abc123"}))
        .send()
        .await
        .expect("push token");
    assert_eq!(resp.status(), StatusCode::NO_CONTENT);

    // Admin list devices
    let resp = client
        .get(format!("{base_url}/api/v1/admin/devices"))
        .bearer_auth(&token)
        .send()
        .await
        .expect("list devices");
    assert_eq!(resp.status(), StatusCode::OK);
    let devices: Value = resp.json().await.unwrap();
    let devices = devices.as_array().unwrap();
    assert!(devices.iter().any(|d| d["id"] == device_id));
    let dev = devices.iter().find(|d| d["id"] == device_id).unwrap();
    assert_eq!(dev["deviceName"], "Test Device");
}

// ---------------------------------------------------------------------------
// Install events
// ---------------------------------------------------------------------------

#[tokio::test]
async fn test_install_event() {
    let (client, base_url, _) = setup().await;

    // Register a device first
    let resp = client
        .post(format!("{base_url}/api/v1/devices/register"))
        .json(&json!({"deviceName": "Event Device"}))
        .send()
        .await
        .expect("register");
    let body: Value = resp.json().await.unwrap();
    let device_id = body["deviceId"].as_str().unwrap();

    // Post install event
    let resp = client
        .post(format!(
            "{base_url}/api/v1/devices/{device_id}/install-event"
        ))
        .json(&json!({
            "packageName": "com.example.app",
            "toVersionCode": 5,
            "status": "success"
        }))
        .send()
        .await
        .expect("install event");
    assert_eq!(resp.status(), StatusCode::CREATED);
}

// ---------------------------------------------------------------------------
// CI token auth
// ---------------------------------------------------------------------------

#[tokio::test]
async fn test_ci_token_upload() {
    let (client, base_url, token) = setup().await;
    let project = create_project(&client, &base_url, &token, "ci-app", "CI App").await;
    let project_id = project["id"].as_str().unwrap();

    // Create CI token scoped to project
    let resp = client
        .post(format!("{base_url}/api/v1/admin/tokens"))
        .bearer_auth(&token)
        .json(&json!({"name": "ci-token", "projectId": project_id}))
        .send()
        .await
        .expect("create token");
    assert_eq!(resp.status(), StatusCode::CREATED);
    let body: Value = resp.json().await.unwrap();
    let ci_token = body["token"].as_str().unwrap().to_string();

    // Upload via CI endpoint using the token
    let apk = create_test_apk("com.test.ci", 1, "1.0.0");
    let part = reqwest::multipart::Part::bytes(apk)
        .file_name("ci.apk")
        .mime_str("application/vnd.android.package-archive")
        .unwrap();
    let form = reqwest::multipart::Form::new().part("apk", part);
    let resp = client
        .post(format!("{base_url}/api/v1/ci/projects/ci-app/release"))
        .bearer_auth(&ci_token)
        .multipart(form)
        .send()
        .await
        .expect("ci upload");
    assert_eq!(resp.status(), StatusCode::CREATED);
    let body: Value = resp.json().await.unwrap();
    assert_eq!(body["release"]["sourceType"], "ci");
}

#[tokio::test]
async fn test_cli_pin_login_token_can_push() {
    use kobup_service::repositories::auth_tokens;

    let (client, base_url, token, pool, _storage_root) = setup_with_pool().await;
    let project = create_project(&client, &base_url, &token, "pin-cli", "Pin CLI").await;
    let _project_id = project["id"].as_str().unwrap();

    // Mint a token the same way pin_login_cli does: an auth_tokens row with
    // scope="ci", not a project-scoped api_tokens row.
    let admin_user_id = {
        let resp = client
            .get(format!("{base_url}/api/v1/admin/me"))
            .bearer_auth(&token)
            .send()
            .await
            .expect("me");
        let body: Value = resp.json().await.unwrap();
        body["userId"].as_str().unwrap().to_string()
    };
    let cli_token = "test-cli-pin-token";
    auth_tokens::create(&pool, &admin_user_id, cli_token, "ci", None)
        .await
        .expect("create ci auth token");

    let apk = create_test_apk("com.test.pincli", 1, "1.0.0");
    let part = reqwest::multipart::Part::bytes(apk)
        .file_name("pincli.apk")
        .mime_str("application/vnd.android.package-archive")
        .unwrap();
    let form = reqwest::multipart::Form::new().part("apk", part);
    let resp = client
        .post(format!("{base_url}/api/v1/ci/projects/pin-cli/release"))
        .bearer_auth(cli_token)
        .multipart(form)
        .send()
        .await
        .expect("ci upload via pin-login token");
    assert_eq!(resp.status(), StatusCode::CREATED);
}

#[tokio::test]
async fn test_ci_token_wrong_project() {
    let (client, base_url, token) = setup().await;

    // Create two projects
    let project_a = create_project(&client, &base_url, &token, "ci-a", "CI A").await;
    let project_a_id = project_a["id"].as_str().unwrap();
    create_project(&client, &base_url, &token, "ci-b", "CI B").await;

    // Create CI token scoped to project A
    let resp = client
        .post(format!("{base_url}/api/v1/admin/tokens"))
        .bearer_auth(&token)
        .json(&json!({"name": "scoped-token", "projectId": project_a_id}))
        .send()
        .await
        .expect("create token");
    let body: Value = resp.json().await.unwrap();
    let ci_token = body["token"].as_str().unwrap().to_string();

    // Try to upload to project B using token scoped to A
    let apk = create_test_apk("com.test.wrong", 1, "1.0.0");
    let part = reqwest::multipart::Part::bytes(apk)
        .file_name("wrong.apk")
        .mime_str("application/vnd.android.package-archive")
        .unwrap();
    let form = reqwest::multipart::Form::new().part("apk", part);
    let resp = client
        .post(format!("{base_url}/api/v1/ci/projects/ci-b/release"))
        .bearer_auth(&ci_token)
        .multipart(form)
        .send()
        .await
        .expect("ci upload wrong project");
    assert_eq!(resp.status(), StatusCode::FORBIDDEN);
}

#[tokio::test]
async fn test_push_refresh_rejects_wrong_project_scope() {
    let (client, base_url, token) = setup().await;

    let project_a = create_project(&client, &base_url, &token, "push-a", "Push A").await;
    let project_a_id = project_a["id"].as_str().unwrap();
    create_project(&client, &base_url, &token, "push-b", "Push B").await;

    let resp = client
        .post(format!("{base_url}/api/v1/admin/tokens"))
        .bearer_auth(&token)
        .json(&json!({"name": "scoped-token", "projectId": project_a_id}))
        .send()
        .await
        .expect("create token");
    let body: Value = resp.json().await.unwrap();
    let ci_token = body["token"].as_str().unwrap().to_string();

    // Scoped to A, refreshing B is forbidden.
    let resp = client
        .post(format!("{base_url}/api/v1/ci/projects/push-b/push-refresh"))
        .bearer_auth(&ci_token)
        .send()
        .await
        .expect("push refresh wrong project");
    assert_eq!(resp.status(), StatusCode::FORBIDDEN);

    // Refreshing its own project still works.
    let resp = client
        .post(format!("{base_url}/api/v1/ci/projects/push-a/push-refresh"))
        .bearer_auth(&ci_token)
        .send()
        .await
        .expect("push refresh own project");
    assert_eq!(resp.status(), StatusCode::OK);

    // An unknown slug is a 404, not a silent no-op.
    let resp = client
        .post(format!(
            "{base_url}/api/v1/ci/projects/does-not-exist/push-refresh"
        ))
        .bearer_auth(&ci_token)
        .send()
        .await
        .expect("push refresh unknown project");
    assert_eq!(resp.status(), StatusCode::NOT_FOUND);
}

// ---------------------------------------------------------------------------
// Token management lifecycle
// ---------------------------------------------------------------------------

#[tokio::test]
async fn test_token_lifecycle() {
    let (client, base_url, token) = setup().await;

    // Create a CI token
    let resp = client
        .post(format!("{base_url}/api/v1/admin/tokens"))
        .bearer_auth(&token)
        .json(&json!({"name": "lifecycle-token"}))
        .send()
        .await
        .expect("create token");
    assert_eq!(resp.status(), StatusCode::CREATED);
    let body: Value = resp.json().await.unwrap();
    let token_id = body["id"].as_str().unwrap().to_string();
    let raw_token = body["token"].as_str().unwrap().to_string();

    // List tokens, verify it appears
    let resp = client
        .get(format!("{base_url}/api/v1/admin/tokens"))
        .bearer_auth(&token)
        .send()
        .await
        .expect("list tokens");
    assert_eq!(resp.status(), StatusCode::OK);
    let tokens: Value = resp.json().await.unwrap();
    let tokens = tokens.as_array().unwrap();
    assert!(tokens.iter().any(|t| t["id"] == token_id));
    let found = tokens.iter().find(|t| t["id"] == token_id).unwrap();
    assert!(found["revokedAt"].is_null());

    // Verify the token works (CI endpoint needs a project, so use a simpler check:
    // try uploading to a non-existent slug - should get NOT_FOUND, not UNAUTHORIZED)
    let apk = create_test_apk("com.test.lifecycle", 1, "1.0.0");
    let part = reqwest::multipart::Part::bytes(apk)
        .file_name("test.apk")
        .mime_str("application/vnd.android.package-archive")
        .unwrap();
    let form = reqwest::multipart::Form::new().part("apk", part);
    let resp = client
        .post(format!("{base_url}/api/v1/ci/projects/nonexistent/release"))
        .bearer_auth(&raw_token)
        .multipart(form)
        .send()
        .await
        .expect("ci with valid token");
    assert_eq!(resp.status(), StatusCode::NOT_FOUND);

    // Revoke the token
    let resp = client
        .delete(format!("{base_url}/api/v1/admin/tokens/{token_id}"))
        .bearer_auth(&token)
        .send()
        .await
        .expect("revoke token");
    assert_eq!(resp.status(), StatusCode::NO_CONTENT);

    // Verify revoked token returns UNAUTHORIZED
    let apk2 = create_test_apk("com.test.lifecycle2", 2, "2.0.0");
    let part = reqwest::multipart::Part::bytes(apk2)
        .file_name("test2.apk")
        .mime_str("application/vnd.android.package-archive")
        .unwrap();
    let form = reqwest::multipart::Form::new().part("apk", part);
    let resp = client
        .post(format!("{base_url}/api/v1/ci/projects/nonexistent/release"))
        .bearer_auth(&raw_token)
        .multipart(form)
        .send()
        .await
        .expect("ci with revoked token");
    assert_eq!(resp.status(), StatusCode::UNAUTHORIZED);
}

// ---------------------------------------------------------------------------
// Channel management
// ---------------------------------------------------------------------------

#[tokio::test]
async fn test_channel_crud() {
    let (client, base_url, token) = setup().await;
    let project = create_project(&client, &base_url, &token, "chan-app", "Chan App").await;
    let project_id = project["id"].as_str().unwrap();

    // Verify auto-created "stable" channel
    let channels = project["channels"].as_array().unwrap();
    assert_eq!(channels.len(), 1);
    assert_eq!(channels[0]["name"], "stable");

    // Create "beta" channel
    let resp = client
        .post(format!(
            "{base_url}/api/v1/admin/projects/{project_id}/channels"
        ))
        .bearer_auth(&token)
        .json(&json!({"name": "beta"}))
        .send()
        .await
        .expect("create beta");
    assert_eq!(resp.status(), StatusCode::CREATED);
    let beta: Value = resp.json().await.unwrap();
    let beta_id = beta["id"].as_str().unwrap();
    assert_eq!(beta["name"], "beta");
    assert!(!beta["isDefault"].as_bool().unwrap());

    // Fetch project to verify both channels
    let resp = client
        .get(format!("{base_url}/api/v1/admin/projects/{project_id}"))
        .bearer_auth(&token)
        .send()
        .await
        .expect("get project");
    assert_eq!(resp.status(), StatusCode::OK);
    let body: Value = resp.json().await.unwrap();
    let channels = body["channels"].as_array().unwrap();
    assert_eq!(channels.len(), 2);
    let names: Vec<&str> = channels
        .iter()
        .map(|c| c["name"].as_str().unwrap())
        .collect();
    assert!(names.contains(&"stable"));
    assert!(names.contains(&"beta"));

    // Delete "beta" channel
    let resp = client
        .delete(format!(
            "{base_url}/api/v1/admin/projects/{project_id}/channels/{beta_id}"
        ))
        .bearer_auth(&token)
        .send()
        .await
        .expect("delete beta");
    assert_eq!(resp.status(), StatusCode::NO_CONTENT);

    // Verify only "stable" remains
    let resp = client
        .get(format!("{base_url}/api/v1/admin/projects/{project_id}"))
        .bearer_auth(&token)
        .send()
        .await
        .expect("get project after delete");
    let body: Value = resp.json().await.unwrap();
    let channels = body["channels"].as_array().unwrap();
    assert_eq!(channels.len(), 1);
    assert_eq!(channels[0]["name"], "stable");
}

// ---------------------------------------------------------------------------
// Catalog search
// ---------------------------------------------------------------------------

#[tokio::test]
async fn test_catalog_search() {
    let (client, base_url, token) = setup().await;

    create_project(&client, &base_url, &token, "my-app", "My App").await;
    create_project(&client, &base_url, &token, "other-tool", "Other Tool").await;

    // Search for "My"
    let resp = client
        .get(format!("{base_url}/api/v1/catalog?search=My"))
        .send()
        .await
        .expect("search");
    assert_eq!(resp.status(), StatusCode::OK);
    let body: Value = resp.json().await.unwrap();
    let items = body.as_array().unwrap();
    assert!(items.iter().any(|i| i["slug"] == "my-app"));
    assert!(!items.iter().any(|i| i["slug"] == "other-tool"));

    // Search for "Other"
    let resp = client
        .get(format!("{base_url}/api/v1/catalog?search=Other"))
        .send()
        .await
        .expect("search other");
    let body: Value = resp.json().await.unwrap();
    let items = body.as_array().unwrap();
    assert!(items.iter().any(|i| i["slug"] == "other-tool"));
    assert!(!items.iter().any(|i| i["slug"] == "my-app"));
}

// ---------------------------------------------------------------------------
// Project delete cleanup
// ---------------------------------------------------------------------------

#[tokio::test]
async fn test_project_delete() {
    let (client, base_url, token) = setup().await;
    let project = create_project(&client, &base_url, &token, "del-app", "Del App").await;
    let project_id = project["id"].as_str().unwrap();

    // Upload an APK
    let apk = create_test_apk("com.test.del", 1, "1.0.0");
    upload_apk(&client, &base_url, &token, project_id, apk).await;

    // Delete project
    let resp = client
        .delete(format!("{base_url}/api/v1/admin/projects/{project_id}"))
        .bearer_auth(&token)
        .send()
        .await
        .expect("delete project");
    assert_eq!(resp.status(), StatusCode::NO_CONTENT);

    // Verify project is gone from list
    let resp = client
        .get(format!("{base_url}/api/v1/admin/projects"))
        .bearer_auth(&token)
        .send()
        .await
        .expect("list projects");
    let body: Value = resp.json().await.unwrap();
    let projects = body.as_array().unwrap();
    assert!(!projects.iter().any(|p| p["id"] == project_id));

    // Verify catalog doesn't include it
    let resp = client
        .get(format!("{base_url}/api/v1/catalog"))
        .send()
        .await
        .expect("catalog");
    let body: Value = resp.json().await.unwrap();
    let items = body.as_array().unwrap();
    assert!(!items.iter().any(|i| i["slug"] == "del-app"));
}

// ---------------------------------------------------------------------------
// Rate limiting
// ---------------------------------------------------------------------------

#[tokio::test]
async fn test_login_rate_limiting() {
    let (client, base_url, _) = setup().await;

    // Rate limiter allows 5 requests per window. setup() already used 1
    // (the successful login), so we have 4 remaining before rate limiting.
    // Send 5 attempts: first 4 should get UNAUTHORIZED, 5th should get 429.
    let mut statuses = Vec::new();
    for _ in 0..5 {
        let resp = client
            .post(format!("{base_url}/api/v1/admin/login"))
            .json(&json!({"email": "admin@test.local", "password": "wrong-password"}))
            .send()
            .await
            .expect("login attempt");
        statuses.push(resp.status());
    }

    // First 4 should be UNAUTHORIZED (budget: 5 total minus 1 from setup = 4 left)
    for status in &statuses[..4] {
        assert_eq!(*status, StatusCode::UNAUTHORIZED);
    }
    // 5th attempt (6th overall) should be TOO_MANY_REQUESTS
    assert_eq!(statuses[4], StatusCode::TOO_MANY_REQUESTS);
}

// ---------------------------------------------------------------------------
// Password change
// ---------------------------------------------------------------------------

#[tokio::test]
async fn test_change_password() {
    let (client, base_url, token) = setup().await;

    let new_password = "new-secure-password-456";

    // Change password
    let resp = client
        .put(format!("{base_url}/api/v1/admin/me/password"))
        .bearer_auth(&token)
        .json(&json!({
            "currentPassword": "test-password-123",
            "newPassword": new_password
        }))
        .send()
        .await
        .expect("change password");
    assert_eq!(resp.status(), StatusCode::NO_CONTENT);

    // Login with new password succeeds
    let resp = client
        .post(format!("{base_url}/api/v1/admin/login"))
        .json(&json!({"email": "admin@test.local", "password": new_password}))
        .send()
        .await
        .expect("login new");
    assert_eq!(resp.status(), StatusCode::OK);

    // Login with old password fails
    let resp = client
        .post(format!("{base_url}/api/v1/admin/login"))
        .json(&json!({"email": "admin@test.local", "password": "test-password-123"}))
        .send()
        .await
        .expect("login old");
    assert_eq!(resp.status(), StatusCode::UNAUTHORIZED);
}

#[tokio::test]
async fn test_change_password_wrong_current() {
    let (client, base_url, token) = setup().await;

    let resp = client
        .put(format!("{base_url}/api/v1/admin/me/password"))
        .bearer_auth(&token)
        .json(&json!({
            "currentPassword": "wrong-current",
            "newPassword": "doesnt-matter-12345"
        }))
        .send()
        .await
        .expect("change password");
    assert_eq!(resp.status(), StatusCode::UNAUTHORIZED);
}

#[tokio::test]
async fn test_change_password_too_short() {
    let (client, base_url, token) = setup().await;

    let resp = client
        .put(format!("{base_url}/api/v1/admin/me/password"))
        .bearer_auth(&token)
        .json(&json!({
            "currentPassword": "test-password-123",
            "newPassword": "short"
        }))
        .send()
        .await
        .expect("change password");
    assert_eq!(resp.status(), StatusCode::BAD_REQUEST);
}

// ---------------------------------------------------------------------------
// Upload with auto-publish
// ---------------------------------------------------------------------------

#[tokio::test]
async fn test_upload_with_auto_publish() {
    let (client, base_url, token) = setup().await;
    let project = create_project(&client, &base_url, &token, "auto-pub", "Auto Pub").await;
    let project_id = project["id"].as_str().unwrap();

    let apk = create_test_apk("com.test.autopub", 1, "1.0.0");
    let part = reqwest::multipart::Part::bytes(apk)
        .file_name("test.apk")
        .mime_str("application/vnd.android.package-archive")
        .unwrap();
    let form = reqwest::multipart::Form::new()
        .part("apk", part)
        .text("publish", "true");

    let resp = client
        .post(format!(
            "{base_url}/api/v1/admin/projects/{project_id}/upload"
        ))
        .bearer_auth(&token)
        .multipart(form)
        .send()
        .await
        .expect("upload");
    assert_eq!(resp.status(), StatusCode::CREATED);
    let body: Value = resp.json().await.unwrap();
    assert!(body["release"]["publishedAt"].is_string());
}

// ---------------------------------------------------------------------------
// Auth: me endpoint and logout
// ---------------------------------------------------------------------------

#[tokio::test]
async fn test_me_endpoint() {
    let (client, base_url, token) = setup().await;

    let resp = client
        .get(format!("{base_url}/api/v1/admin/me"))
        .bearer_auth(&token)
        .send()
        .await
        .expect("me");
    assert_eq!(resp.status(), StatusCode::OK);
    let body: Value = resp.json().await.unwrap();
    assert_eq!(body["email"], "admin@test.local");
    assert_eq!(body["role"], "admin");
    assert!(body["userId"].is_string());
}

#[tokio::test]
async fn test_logout() {
    let (client, base_url, token) = setup().await;

    // Logout
    let resp = client
        .post(format!("{base_url}/api/v1/admin/logout"))
        .bearer_auth(&token)
        .send()
        .await
        .expect("logout");
    assert_eq!(resp.status(), StatusCode::NO_CONTENT);

    // Token should now be invalid
    let resp = client
        .get(format!("{base_url}/api/v1/admin/me"))
        .bearer_auth(&token)
        .send()
        .await
        .expect("me after logout");
    assert_eq!(resp.status(), StatusCode::UNAUTHORIZED);
}

// ---------------------------------------------------------------------------
// Latest by slug (public endpoint)
// ---------------------------------------------------------------------------

#[tokio::test]
async fn test_latest_by_slug() {
    let (client, base_url, token) = setup().await;
    let project = create_project(&client, &base_url, &token, "latest-app", "Latest App").await;
    let project_id = project["id"].as_str().unwrap();

    // No published release yet
    let resp = client
        .get(format!("{base_url}/api/v1/projects/latest-app/latest"))
        .send()
        .await
        .expect("latest empty");
    assert_eq!(resp.status(), StatusCode::NOT_FOUND);

    // Upload and publish
    let apk = create_test_apk("com.test.latest", 10, "2.0.0");
    let part = reqwest::multipart::Part::bytes(apk)
        .file_name("test.apk")
        .mime_str("application/vnd.android.package-archive")
        .unwrap();
    let form = reqwest::multipart::Form::new()
        .part("apk", part)
        .text("publish", "true");
    let resp = client
        .post(format!(
            "{base_url}/api/v1/admin/projects/{project_id}/upload"
        ))
        .bearer_auth(&token)
        .multipart(form)
        .send()
        .await
        .expect("upload");
    assert_eq!(resp.status(), StatusCode::CREATED);

    // Now latest should return the release
    let resp = client
        .get(format!("{base_url}/api/v1/projects/latest-app/latest"))
        .send()
        .await
        .expect("latest");
    assert_eq!(resp.status(), StatusCode::OK);
    let body: Value = resp.json().await.unwrap();
    assert_eq!(body["versionCode"], 10);
    assert_eq!(body["versionName"], "2.0.0");
    assert_eq!(body["projectSlug"], "latest-app");
    assert!(body["downloadUrl"].is_string());
    assert!(body["sha256"].is_string());
}

// ---------------------------------------------------------------------------
// Delete release
// ---------------------------------------------------------------------------

#[tokio::test]
async fn test_delete_release() {
    let (client, base_url, token, pool, _storage_root) = setup_with_pool().await;
    let project = create_project(&client, &base_url, &token, "del-rel", "Del Rel").await;
    let project_id = project["id"].as_str().unwrap();

    let apk = create_test_apk("com.test.delrel", 1, "1.0.0");
    let upload_resp = upload_apk(&client, &base_url, &token, project_id, apk).await;
    let release_id = upload_resp["release"]["id"].as_str().unwrap();

    // The on-disk path the upload actually wrote to — fetched from the DB rather
    // than reconstructed, so this test doesn't assume the same ID scheme the
    // delete path might (incorrectly) assume.
    let file_path: String =
        sqlx::query_scalar("SELECT file_path_or_object_key FROM artifacts WHERE release_id = ?")
            .bind(release_id)
            .fetch_one(&pool)
            .await
            .expect("artifact row");
    assert!(
        std::path::Path::new(&file_path).exists(),
        "uploaded file should exist before delete"
    );

    // Delete the release
    let resp = client
        .delete(format!("{base_url}/api/v1/admin/releases/{release_id}"))
        .bearer_auth(&token)
        .send()
        .await
        .expect("delete release");
    assert_eq!(resp.status(), StatusCode::NO_CONTENT);

    // Verify release is gone
    let resp = client
        .get(format!(
            "{base_url}/api/v1/admin/projects/{project_id}/releases"
        ))
        .bearer_auth(&token)
        .send()
        .await
        .expect("list releases");
    let releases: Value = resp.json().await.unwrap();
    assert!(releases.as_array().unwrap().is_empty());

    // The regression this guards: artifact creation used to mint its own random
    // DB-row ID independent of the one already used for the storage folder, so
    // every delete path silently removed a folder that never existed and the
    // file leaked on disk forever.
    assert!(
        !std::path::Path::new(&file_path).exists(),
        "deleting the release should have removed its artifact file from disk"
    );
}

#[tokio::test]
async fn test_delete_project_removes_artifact_files_from_disk() {
    let (client, base_url, token, pool, _storage_root) = setup_with_pool().await;
    let project = create_project(&client, &base_url, &token, "del-proj", "Del Proj").await;
    let project_id = project["id"].as_str().unwrap();

    let apk = create_test_apk("com.test.delproj", 1, "1.0.0");
    let upload_resp = upload_apk(&client, &base_url, &token, project_id, apk).await;
    let release_id = upload_resp["release"]["id"].as_str().unwrap();

    let file_path: String =
        sqlx::query_scalar("SELECT file_path_or_object_key FROM artifacts WHERE release_id = ?")
            .bind(release_id)
            .fetch_one(&pool)
            .await
            .expect("artifact row");
    assert!(std::path::Path::new(&file_path).exists());

    let resp = client
        .delete(format!("{base_url}/api/v1/admin/projects/{project_id}"))
        .bearer_auth(&token)
        .send()
        .await
        .expect("delete project");
    assert_eq!(resp.status(), StatusCode::NO_CONTENT);

    assert!(
        !std::path::Path::new(&file_path).exists(),
        "deleting the project should have removed its artifact file from disk"
    );
}

#[test]
fn apk_metadata_extraction_reads_manifest_abis_and_signing_block() {
    let manifest = build_binary_manifest("com.kobup.example", 42, "1.2.3");
    let signing_block = b"not-a-real-pkcs7-blob".to_vec();

    let mut zip = zip::ZipWriter::new(std::io::Cursor::new(Vec::new()));
    let opts = zip::write::SimpleFileOptions::default();
    zip.start_file::<_, ()>("AndroidManifest.xml", opts)
        .unwrap();
    zip.write_all(&manifest).unwrap();
    zip.start_file::<_, ()>("classes.dex", opts).unwrap();
    zip.write_all(b"fake dex").unwrap();
    for abi in ["arm64-v8a", "x86_64"] {
        zip.start_file::<_, ()>(format!("lib/{abi}/libkobup.so"), opts)
            .unwrap();
        zip.write_all(b"fake so").unwrap();
    }
    zip.start_file::<_, ()>("META-INF/CERT.RSA", opts).unwrap();
    zip.write_all(&signing_block).unwrap();
    let apk = zip.finish().unwrap().into_inner();

    let meta = kobup_service::apk::extract_metadata(&apk).expect("extract_metadata");

    assert_eq!(meta.package_name.as_deref(), Some("com.kobup.example"));
    assert_eq!(meta.version_code, Some(42));
    assert_eq!(meta.version_name.as_deref(), Some("1.2.3"));
    assert_eq!(
        meta.abis,
        vec!["arm64-v8a".to_string(), "x86_64".to_string()]
    );
    assert_eq!(
        meta.signing_fingerprint.as_deref(),
        Some(kobup_service::apk::compute_sha256(&signing_block).as_str())
    );
}

#[tokio::test]
async fn test_pin_login_publishes_to_configured_pin_service() {
    let received: Arc<std::sync::Mutex<Option<(String, Value)>>> = Default::default();
    let sink = received.clone();
    let relay = axum::Router::new().route(
        "/pin/kobup/{pin}",
        axum::routing::put(
            move |axum::extract::Path(pin): axum::extract::Path<String>,
                  axum::Json(body): axum::Json<Value>| async move {
                *sink.lock().unwrap() = Some((pin, body));
                StatusCode::ACCEPTED
            },
        ),
    );
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0")
        .await
        .expect("bind relay");
    let relay_url = format!("http://{}", listener.local_addr().unwrap());
    tokio::spawn(async move { axum::serve(listener, relay).await.ok() });

    let (client, base_url, token, _pool, _root) =
        setup_with(|c| c.pin_service_url = relay_url).await;

    let resp = client
        .post(format!("{base_url}/api/v1/admin/pin-login/device"))
        .bearer_auth(&token)
        .json(&json!({"pin": "abcd12", "api_url": "https://updates.example/"}))
        .send()
        .await
        .expect("pin login");
    assert_eq!(resp.status(), StatusCode::NO_CONTENT);

    let (pin, body) = received.lock().unwrap().take().expect("relay was called");
    assert_eq!(pin, "ABCD12");
    assert_eq!(body["api_url"], "https://updates.example");
}
