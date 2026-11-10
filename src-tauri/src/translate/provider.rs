//! 翻译服务商抽象与五个 preset 实现（百度 / 讯飞 / DeepL / 有道 / Google）。
//!
//! 第三方接口要点均核对自官方接入文档后写入（baseline §G）：
//! - 百度：`POST fanyi-api.baidu.com/api/trans/vip/translate` 表单 q/from/to/appid/salt/sign，
//!   `sign = md5(appid + q + salt + 密钥)`，返回 `trans_result[].dst`，失败带 `error_code`；
//! - DeepL：`POST /v2/translate`，`Authorization: DeepL-Auth-Key`，JSON `text` / `target_lang`，
//!   返回 `translations[].text`；免费 Key（后缀 `:fx`）走 `api-free` 域名；
//! - 有道：`POST openapi.youdao.com/api` 表单 q/from/to/appKey/salt/sign/signType=v3/curtime，
//!   `sign = sha256(appKey + input + salt + curtime + 应用密钥)`，返回 `translation[]` / `errorCode`；
//! - Google：`POST translation.googleapis.com/language/translate/v2?key=`，JSON q/source/target/format，
//!   返回 `data.translations[].translatedText`；
//! - 讯飞机器翻译（新）：`POST itrans.xf-yun.com/v1/its`，URL query 带 `host/date/authorization`，
//!   签名为 `base64(HMAC-SHA256(APISecret, "host: ..\ndate: ..\nPOST /v1/its HTTP/1.1"))`，
//!   请求体 `payload.input_data.text` 为原文 base64，响应 `payload.result.text` 为 base64(JSON)，
//!   取 `trans_result.dst`；中文语种代码是 `cn` 而非 `zh`。
//!
//! 红线：错误只带 HTTP 状态或官方错误码，**不回显响应体 / Key / 原文 / 译文**；
//! `reqwest::Error` 不得用 `?` 直接转换——其 Display 可能带含 Key 的 URL。

use std::sync::OnceLock;

use async_trait::async_trait;
use base64::Engine as _;
use base64::engine::general_purpose::STANDARD as B64;
use hmac::{Hmac, Mac};
use md5::{Digest as _, Md5};
use serde_json::Value;
use sha2::Sha256;

use crate::error::AppError;

/// 本版仅中英互译的语种枚举。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Lang {
    Zh,
    En,
}

impl Lang {
    /// 百度 / 通用语言代码。
    pub fn as_str(self) -> &'static str {
        match self {
            Lang::Zh => "zh",
            Lang::En => "en",
        }
    }

    /// 自动互译的目标语言（与源语言相反）。
    pub fn opposite(self) -> Lang {
        match self {
            Lang::Zh => Lang::En,
            Lang::En => Lang::Zh,
        }
    }

    /// Google v2 语言代码（简体中文用 `zh-CN`）。
    pub fn google_code(self) -> &'static str {
        match self {
            Lang::Zh => "zh-CN",
            Lang::En => "en",
        }
    }

    /// DeepL 语言代码（大写）。
    pub fn deepl_code(self) -> &'static str {
        match self {
            Lang::Zh => "ZH",
            Lang::En => "EN",
        }
    }

    /// 有道语言代码。
    pub fn youdao_code(self) -> &'static str {
        match self {
            Lang::Zh => "zh-CHS",
            Lang::En => "en",
        }
    }

    /// 讯飞机器翻译（新）语言代码：中文是 `cn`，英文是 `en`。
    pub fn xfyun_code(self) -> &'static str {
        match self {
            Lang::Zh => "cn",
            Lang::En => "en",
        }
    }

    /// 解析服务商返回的检测语种；仅接受中英常见别名，其它语言返回 None（本版不支持）。
    pub fn parse(raw: &str) -> Option<Lang> {
        match raw.to_ascii_lowercase().as_str() {
            "zh" | "cn" | "zh-cn" | "zh-chs" | "zh-hans" | "chi" => Some(Lang::Zh),
            "en" | "en-us" | "en-gb" => Some(Lang::En),
            _ => None,
        }
    }
}

/// 翻译请求（凭据为借用，避免复制明文到多处）。
#[derive(Debug, Clone, Copy)]
pub struct TranslateRequest<'a> {
    pub text: &'a str,
    pub source: Lang,
    pub target: Lang,
    /// 百度 / 有道 / 讯飞的应用 ID；DeepL / Google 传空串（只用 `secret`）。
    pub app_id: &'a str,
    /// 签名密钥：百度密钥 / 有道应用密钥 / 讯飞 APISecret；DeepL / Google 的 API Key。
    pub secret: &'a str,
    /// 仅讯飞需要的 APIKey（参与 `authorization_origin`）；其余服务商传空串。
    pub api_key: &'a str,
}

/// 服务商返回（检测语种可选，用于纠正自动互译的源语言判定）。
#[derive(Debug, Clone)]
pub struct TranslationOutcome {
    pub translated_text: String,
    pub detected_source: Option<Lang>,
}

/// 服务商抽象：每个 preset 一个实现，便于单测用桩替换。
#[async_trait]
pub trait TranslateProvider: Send + Sync {
    /// 与 `translate_settings.provider` 一致的服务商标识。
    fn id(&self) -> &'static str;
    /// 是否计入内置额度（仅内置百度通道为 true）。
    fn is_builtin(&self) -> bool;
    /// 发起一次翻译；错误一律 `AppError::Llm` / `AppError::Config`，不含敏感信息。
    async fn translate(
        &self,
        http: &reqwest::Client,
        req: TranslateRequest<'_>,
    ) -> Result<TranslationOutcome, AppError>;
}

/// 全应用复用一个 reqwest 连接池（rustls TLS；连接/总超时来自 config）。
pub fn http_client() -> &'static reqwest::Client {
    static CLIENT: OnceLock<reqwest::Client> = OnceLock::new();
    CLIENT.get_or_init(|| {
        reqwest::Client::builder()
            .connect_timeout(std::time::Duration::from_secs(
                crate::config::TRANSLATE_CONNECT_TIMEOUT_SECS,
            ))
            .timeout(std::time::Duration::from_secs(crate::config::TRANSLATE_TIMEOUT_SECS))
            .build()
            .unwrap_or_default()
    })
}

/// 网络错误分类：只带类型 / 状态码，**绝不打印 `reqwest::Error`（可能含 Key 的 URL）**。
fn network_error(e: reqwest::Error) -> AppError {
    if e.is_timeout() {
        AppError::Llm("翻译请求超时".into())
    } else if e.is_connect() {
        AppError::Llm("翻译服务连接失败".into())
    } else if let Some(status) = e.status() {
        AppError::Llm(format!("翻译服务 HTTP {}", status.as_u16()))
    } else {
        AppError::Llm("翻译请求失败".into())
    }
}

/// HTTP 非 2xx：只回传状态码，不回传响应体。
fn status_error(service: &str, status: reqwest::StatusCode) -> AppError {
    AppError::Llm(format!("{service} HTTP {}", status.as_u16()))
}

/// 错误码字段的字符串化（有道 / 百度可能给字符串或数字）。
fn error_code_of(body: &Value, field: &str) -> String {
    match body.get(field) {
        Some(Value::String(s)) => s.clone(),
        Some(other) => other.to_string(),
        None => "unknown".into(),
    }
}

fn md5_hex(input: &str) -> String {
    let mut hasher = Md5::new();
    hasher.update(input.as_bytes());
    hasher.finalize().iter().map(|b| format!("{b:02x}")).collect()
}

fn sha256_hex(input: &str) -> String {
    let mut hasher = Sha256::new();
    hasher.update(input.as_bytes());
    hasher.finalize().iter().map(|b| format!("{b:02x}")).collect()
}

fn unix_secs() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or_default()
}

/// 随机串（8 字节随机 + 秒级时间戳），用于百度 salt / 有道防重放。
fn random_salt() -> String {
    let mut buf = [0u8; 8];
    let _ = getrandom::getrandom(&mut buf);
    let mut salt: String = buf.iter().map(|b| format!("{b:02x}")).collect();
    salt.push_str(&unix_secs().to_string());
    salt
}

/// 有道 v3 签名的 `input`：q 超过 20 字符时取「前 10 + 长度 + 后 10」。
fn youdao_sign_input(q: &str) -> String {
    let chars: Vec<char> = q.chars().collect();
    if chars.len() <= 20 {
        return q.to_string();
    }
    let head: String = chars.iter().take(10).collect();
    let tail: String = chars.iter().skip(chars.len() - 10).collect();
    format!("{head}{}{tail}", chars.len())
}

/// 从 JSON 数组字段收集字符串（各服务商译文形状统一在这里归一化）。
fn collect_strings(body: &Value, field: &str, item_field: Option<&str>) -> Vec<String> {
    let Some(items) = body.get(field).and_then(Value::as_array) else {
        return Vec::new();
    };
    items
        .iter()
        .filter_map(|item| match item_field {
            Some(key) => item.get(key).and_then(Value::as_str).map(str::to_string),
            None => item.as_str().map(str::to_string),
        })
        .collect()
}

// ---------- 内置零配置通道：百度翻译 ----------

/// 百度通用翻译 API（内置凭据，计入内置额度）。
pub struct BaiduProvider;

#[async_trait]
impl TranslateProvider for BaiduProvider {
    fn id(&self) -> &'static str {
        "baidu"
    }

    fn is_builtin(&self) -> bool {
        true
    }

    async fn translate(
        &self,
        http: &reqwest::Client,
        req: TranslateRequest<'_>,
    ) -> Result<TranslationOutcome, AppError> {
        let salt = random_salt();
        let sign = md5_hex(&format!("{}{}{}{}", req.app_id, req.text, salt, req.secret));
        let form = [
            ("q", req.text),
            ("from", req.source.as_str()),
            ("to", req.target.as_str()),
            ("appid", req.app_id),
            ("salt", salt.as_str()),
            ("sign", sign.as_str()),
        ];
        let resp = http
            .post(crate::config::BAIDU_TRANSLATE_URL)
            .form(&form)
            .send()
            .await
            .map_err(network_error)?;
        let status = resp.status();
        if !status.is_success() {
            return Err(status_error("百度翻译", status));
        }
        let body: Value = resp.json().await.map_err(network_error)?;
        if body.get("error_code").is_some() {
            return Err(AppError::Llm(format!(
                "百度翻译错误码 {}",
                error_code_of(&body, "error_code")
            )));
        }
        let translated_text = collect_strings(&body, "trans_result", Some("dst")).join("\n");
        if translated_text.is_empty() {
            return Err(AppError::Llm("百度翻译返回空结果".into()));
        }
        Ok(TranslationOutcome {
            translated_text,
            detected_source: body.get("from").and_then(Value::as_str).and_then(Lang::parse),
        })
    }
}

// ---------- 内置零配置通道：讯飞机器翻译（新） ----------

/// RFC1123 GMT 时间戳（服务端允许 ±300 秒时钟偏移）。
fn rfc1123_gmt_now() -> String {
    chrono::Utc::now().format("%a, %d %b %Y %H:%M:%S GMT").to_string()
}

/// 生成讯飞 `authorization` query 参数：`base64(authorization_origin)`。
///
/// 官方文档口径：`signature_origin = "host: {host}\ndate: {date}\n{request-line}"`，
/// 以 APISecret 为密钥做 HMAC-SHA256 后 base64 得 `signature`；再拼
/// `authorization_origin`（含 APIKey）整体 base64。
fn xf_authorization(
    date: &str,
    host: &str,
    api_key: &str,
    api_secret: &str,
) -> Result<String, AppError> {
    let origin =
        format!("host: {host}\ndate: {date}\n{}", crate::config::XF_TRANSLATE_REQUEST_LINE);
    let mut mac = Hmac::<Sha256>::new_from_slice(api_secret.as_bytes())
        .map_err(|_| AppError::Config("讯飞签名密钥无效".into()))?;
    mac.update(origin.as_bytes());
    let signature = B64.encode(mac.finalize().into_bytes());
    let authorization_origin = format!(
        "api_key=\"{api_key}\",algorithm=\"hmac-sha256\",headers=\"host date request-line\",signature=\"{signature}\""
    );
    Ok(B64.encode(authorization_origin.as_bytes()))
}

/// 讯飞机器翻译（新）API（内置凭据，计入内置额度）。
pub struct XunfeiProvider;

#[async_trait]
impl TranslateProvider for XunfeiProvider {
    fn id(&self) -> &'static str {
        "xfyun"
    }

    fn is_builtin(&self) -> bool {
        true
    }

    async fn translate(
        &self,
        http: &reqwest::Client,
        req: TranslateRequest<'_>,
    ) -> Result<TranslationOutcome, AppError> {
        let host = crate::config::XF_TRANSLATE_HOST;
        let date = rfc1123_gmt_now();
        let authorization = xf_authorization(&date, host, req.api_key, req.secret)?;
        let body = serde_json::json!({
            "header": { "app_id": req.app_id, "status": 3 },
            "parameter": {
                "its": {
                    "from": req.source.xfyun_code(),
                    "to": req.target.xfyun_code(),
                    "result": {}
                }
            },
            "payload": {
                "input_data": {
                    "encoding": "utf8",
                    "status": 3,
                    "text": B64.encode(req.text.as_bytes())
                }
            }
        });
        // 鉴权参数全走 query；reqwest 负责 URL 编码（date 含空格与逗号）。
        let resp = http
            .post(crate::config::XF_TRANSLATE_URL)
            .query(&[
                ("authorization", authorization.as_str()),
                ("host", host),
                ("date", date.as_str()),
            ])
            .json(&body)
            .send()
            .await
            .map_err(network_error)?;
        let status = resp.status();
        if !status.is_success() {
            return Err(status_error("讯飞翻译", status));
        }
        let body: Value = resp.json().await.map_err(network_error)?;
        let code =
            body.get("header").and_then(|h| h.get("code")).and_then(Value::as_i64).unwrap_or(-1);
        if code != 0 {
            return Err(AppError::Llm(format!("讯飞翻译错误码 {code}")));
        }
        let encoded = body
            .get("payload")
            .and_then(|p| p.get("result"))
            .and_then(|r| r.get("text"))
            .and_then(Value::as_str)
            .unwrap_or("");
        let decoded =
            B64.decode(encoded).map_err(|_| AppError::Llm("讯飞翻译响应解码失败".into()))?;
        let inner: Value = serde_json::from_slice(&decoded)
            .map_err(|_| AppError::Llm("讯飞翻译响应解析失败".into()))?;
        let translated_text = inner
            .get("trans_result")
            .and_then(|t| t.get("dst"))
            .and_then(Value::as_str)
            .unwrap_or("")
            .to_string();
        if translated_text.is_empty() {
            return Err(AppError::Llm("讯飞翻译返回空结果".into()));
        }
        let detected_source = inner.get("from").and_then(Value::as_str).and_then(Lang::parse);
        Ok(TranslationOutcome { translated_text, detected_source })
    }
}

// ---------- BYOK 预设：DeepL ----------

/// DeepL API（免费 Key 以 `:fx` 结尾，走 api-free 域名）。
pub struct DeeplProvider;

#[async_trait]
impl TranslateProvider for DeeplProvider {
    fn id(&self) -> &'static str {
        "deepl"
    }

    fn is_builtin(&self) -> bool {
        false
    }

    async fn translate(
        &self,
        http: &reqwest::Client,
        req: TranslateRequest<'_>,
    ) -> Result<TranslationOutcome, AppError> {
        let endpoint = if req.secret.ends_with(":fx") {
            crate::config::DEEPL_TRANSLATE_FREE_URL
        } else {
            crate::config::DEEPL_TRANSLATE_PRO_URL
        };
        let body = serde_json::json!({
            "text": [req.text],
            "source_lang": req.source.deepl_code(),
            "target_lang": req.target.deepl_code(),
        });
        let resp = http
            .post(endpoint)
            .header("Authorization", format!("DeepL-Auth-Key {}", req.secret))
            .json(&body)
            .send()
            .await
            .map_err(network_error)?;
        let status = resp.status();
        if !status.is_success() {
            return Err(status_error("DeepL", status));
        }
        let body: Value = resp.json().await.map_err(network_error)?;
        let translated_text = collect_strings(&body, "translations", Some("text")).join("\n");
        if translated_text.is_empty() {
            return Err(AppError::Llm("DeepL 返回空结果".into()));
        }
        let detected_source = body
            .get("translations")
            .and_then(Value::as_array)
            .and_then(|items| items.first())
            .and_then(|item| item.get("detected_source_language"))
            .and_then(Value::as_str)
            .and_then(Lang::parse);
        Ok(TranslationOutcome { translated_text, detected_source })
    }
}

// ---------- BYOK 预设：有道智云 ----------

/// 有道智云文本翻译 API（v3 SHA256 签名）。
pub struct YoudaoProvider;

#[async_trait]
impl TranslateProvider for YoudaoProvider {
    fn id(&self) -> &'static str {
        "youdao"
    }

    fn is_builtin(&self) -> bool {
        false
    }

    async fn translate(
        &self,
        http: &reqwest::Client,
        req: TranslateRequest<'_>,
    ) -> Result<TranslationOutcome, AppError> {
        let curtime = unix_secs().to_string();
        let salt = random_salt();
        let input = youdao_sign_input(req.text);
        let sign = sha256_hex(&format!("{}{}{}{}{}", req.app_id, input, salt, curtime, req.secret));
        let form = [
            ("q", req.text),
            ("from", req.source.youdao_code()),
            ("to", req.target.youdao_code()),
            ("appKey", req.app_id),
            ("salt", salt.as_str()),
            ("sign", sign.as_str()),
            ("signType", "v3"),
            ("curtime", curtime.as_str()),
        ];
        let resp = http
            .post(crate::config::YOUDAO_TRANSLATE_URL)
            .form(&form)
            .send()
            .await
            .map_err(network_error)?;
        let status = resp.status();
        if !status.is_success() {
            return Err(status_error("有道翻译", status));
        }
        let body: Value = resp.json().await.map_err(network_error)?;
        let code = error_code_of(&body, "errorCode");
        if code != "0" {
            return Err(AppError::Llm(format!("有道翻译错误码 {code}")));
        }
        let translated_text = collect_strings(&body, "translation", None).join("\n");
        if translated_text.is_empty() {
            return Err(AppError::Llm("有道翻译返回空结果".into()));
        }
        Ok(TranslationOutcome { translated_text, detected_source: None })
    }
}

// ---------- BYOK 预设：Google Cloud Translation v2 ----------

/// Google Cloud Translation v2（API Key 走 query 参数，故错误绝不回显 URL）。
pub struct GoogleProvider;

#[async_trait]
impl TranslateProvider for GoogleProvider {
    fn id(&self) -> &'static str {
        "google"
    }

    fn is_builtin(&self) -> bool {
        false
    }

    async fn translate(
        &self,
        http: &reqwest::Client,
        req: TranslateRequest<'_>,
    ) -> Result<TranslationOutcome, AppError> {
        let body = serde_json::json!({
            "q": req.text,
            "source": req.source.google_code(),
            "target": req.target.google_code(),
            "format": "text",
        });
        let resp = http
            .post(crate::config::GOOGLE_TRANSLATE_URL)
            .query(&[("key", req.secret)])
            .json(&body)
            .send()
            .await
            .map_err(network_error)?;
        let status = resp.status();
        if !status.is_success() {
            return Err(status_error("Google 翻译", status));
        }
        let body: Value = resp.json().await.map_err(network_error)?;
        let translated_text = body
            .get("data")
            .and_then(|data| data.get("translations"))
            .map(|items| {
                collect_strings(
                    &serde_json::json!({ "items": items }),
                    "items",
                    Some("translatedText"),
                )
            })
            .unwrap_or_default()
            .join("\n");
        if translated_text.is_empty() {
            return Err(AppError::Llm("Google 翻译返回空结果".into()));
        }
        Ok(TranslationOutcome { translated_text, detected_source: None })
    }
}

/// 按服务商标识构造 preset 实现。
pub fn provider_for(id: &str) -> Result<Box<dyn TranslateProvider>, AppError> {
    match id {
        "baidu" => Ok(Box::new(BaiduProvider)),
        "xfyun" => Ok(Box::new(XunfeiProvider)),
        "deepl" => Ok(Box::new(DeeplProvider)),
        "youdao" => Ok(Box::new(YoudaoProvider)),
        "google" => Ok(Box::new(GoogleProvider)),
        other => Err(AppError::Config(format!("未知翻译服务商: {other}"))),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn md5_hex_matches_known_vector() {
        assert_eq!(md5_hex("hello"), "5d41402abc4b2a76b9719d911017c592");
    }

    #[test]
    fn sha256_hex_matches_known_vector() {
        assert_eq!(
            sha256_hex("abc"),
            "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad"
        );
    }

    #[test]
    fn youdao_sign_input_truncates_long_text() {
        assert_eq!(youdao_sign_input("short"), "short");
        let q = "0123456789abcdefghijKLMNOPQRST"; // 30 字符
        assert_eq!(youdao_sign_input(q), "012345678930KLMNOPQRST");
    }

    #[test]
    fn lang_parse_accepts_zh_en_aliases_only() {
        assert_eq!(Lang::parse("zh-CHS"), Some(Lang::Zh));
        assert_eq!(Lang::parse("cn"), Some(Lang::Zh), "讯飞中文代码是 cn");
        assert_eq!(Lang::parse("EN-US"), Some(Lang::En));
        assert_eq!(Lang::parse("ja"), None);
        assert_eq!(Lang::Zh.opposite(), Lang::En);
        assert_eq!(Lang::En.opposite(), Lang::Zh);
        assert_eq!(Lang::Zh.xfyun_code(), "cn");
        assert_eq!(Lang::En.xfyun_code(), "en");
    }

    #[test]
    fn xf_authorization_is_deterministic_and_well_formed() {
        let date = "Thu, 18 Nov 2021 03:05:18 GMT";
        let host = crate::config::XF_TRANSLATE_HOST;
        let a = xf_authorization(date, host, "apikey-1", "apisecret-1").expect("签名");
        assert_eq!(a, xf_authorization(date, host, "apikey-1", "apisecret-1").expect("同参一致"));
        assert_ne!(a, xf_authorization(date, host, "apikey-1", "apisecret-2").expect("换密钥变化"));
        let origin =
            String::from_utf8(B64.decode(&a).expect("authorization 为 base64")).expect("UTF-8");
        assert!(origin.contains("api_key=\"apikey-1\""));
        assert!(origin.contains("algorithm=\"hmac-sha256\""));
        assert!(origin.contains("headers=\"host date request-line\""));
        let signature =
            origin.split("signature=\"").nth(1).and_then(|s| s.strip_suffix('"')).unwrap_or("");
        assert_eq!(signature.len(), 44, "HMAC-SHA256 的 base64 长度为 44");
    }

    #[test]
    fn xf_rfc1123_date_is_gmt_formatted() {
        let date = rfc1123_gmt_now();
        assert!(date.ends_with(" GMT"), "必须 GMT 结尾: {date}");
        assert!(date.contains(", "), "RFC1123 需含星期与逗号: {date}");
        assert_eq!(date.len(), 29, "形如 Thu, 18 Nov 2021 03:05:18 GMT");
    }

    #[test]
    fn provider_for_rejects_unknown_id() {
        assert!(provider_for("bing").is_err());
        assert_eq!(provider_for("baidu").expect("百度").id(), "baidu");
        assert!(provider_for("baidu").expect("百度").is_builtin());
        assert_eq!(provider_for("xfyun").expect("讯飞").id(), "xfyun");
        assert!(provider_for("xfyun").expect("讯飞").is_builtin());
        assert!(!provider_for("deepl").expect("DeepL").is_builtin());
    }
}
