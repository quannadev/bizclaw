//! Bank BIN resolving and VietQR transfer image generation via qr.sepay.vn.

use bizclaw_core::error::{BizClawError, Result};
use std::io::Write;
use std::path::PathBuf;

pub fn resolve_bank_bin(input: &str) -> Option<String> {
    let cleaned = input.to_lowercase().replace([' ', '_'], "");
    match cleaned.as_str() {
        "abbank" => Some("970425".into()),
        "acb" => Some("970416".into()),
        "agribank" => Some("970405".into()),
        "bidv" => Some("970418".into()),
        "bvbank" => Some("970454".into()),
        "bacabank" => Some("970409".into()),
        "baoviet" => Some("970438".into()),
        "cake" => Some("546034".into()),
        "cbbank" => Some("970444".into()),
        "cimb" => Some("422589".into()),
        "coopbank" => Some("970446".into()),
        "dbs" => Some("796500".into()),
        "dongabank" => Some("970406".into()),
        "eximbank" => Some("970431".into()),
        "gpbank" => Some("970408".into()),
        "hdbank" => Some("970437".into()),
        "hsbc" => Some("458761".into()),
        "hongleong" => Some("970442".into()),
        "ibkhcm" => Some("970456".into()),
        "ibkhn" => Some("970455".into()),
        "indovina" => Some("970434".into()),
        "kbank" => Some("668888".into()),
        "kienlongbank" => Some("970452".into()),
        "kookminhcm" => Some("970463".into()),
        "kookminhn" => Some("970462".into()),
        "mbbank" | "mb" => Some("970422".into()),
        "msb" => Some("970426".into()),
        "ncb" => Some("970419".into()),
        "namabank" => Some("970428".into()),
        "nonghyup" => Some("801011".into()),
        "ocb" => Some("970448".into()),
        "oceanbank" => Some("970414".into()),
        "pgbank" => Some("970430".into()),
        "pvcombank" => Some("970412".into()),
        "publicbank" => Some("970439".into()),
        "scb" => Some("970429".into()),
        "shb" => Some("970443".into()),
        "sacombank" => Some("970403".into()),
        "saigonbank" => Some("970400".into()),
        "seabank" => Some("970440".into()),
        "shinhan" => Some("970424".into()),
        "standardchartered" => Some("970410".into()),
        "tnex" => Some("9704261".into()),
        "tpbank" => Some("970423".into()),
        "techcombank" => Some("970407".into()),
        "timo" => Some("963388".into()),
        "ubank" => Some("546035".into()),
        "uob" => Some("970458".into()),
        "vib" => Some("970441".into()),
        "vpbank" => Some("970432".into()),
        "vrb" => Some("970421".into()),
        "vietabank" => Some("970427".into()),
        "vietbank" => Some("970433".into()),
        "vietcombank" | "vcb" => Some("970436".into()),
        "vietinbank" | "ctg" => Some("970415".into()),
        "woori" => Some("970457".into()),
        _ => {
            if cleaned.chars().all(|c| c.is_ascii_digit()) {
                Some(cleaned)
            } else {
                None
            }
        }
    }
}

pub fn bin_to_sepay(bin: &str) -> Option<&'static str> {
    match bin {
        "970415" => Some("VietinBank"),
        "970436" => Some("Vietcombank"),
        "970422" => Some("MBBank"),
        "970416" => Some("ACB"),
        "970432" => Some("VPBank"),
        "970423" => Some("TPBank"),
        "970426" => Some("MSB"),
        "970428" => Some("NamABank"),
        "970449" => Some("LienVietPostBank"),
        "970454" => Some("VietCapitalBank"),
        "970418" => Some("BIDV"),
        "970403" => Some("Sacombank"),
        "970441" => Some("VIB"),
        "970437" => Some("HDBank"),
        "970440" => Some("SeABank"),
        "970408" => Some("GPBank"),
        "970412" => Some("PVcomBank"),
        "970419" => Some("NCB"),
        "970424" => Some("ShinhanBank"),
        "970429" => Some("SCB"),
        "970430" => Some("PGBank"),
        "970405" => Some("Agribank"),
        "970407" => Some("Techcombank"),
        "970400" => Some("SaigonBank"),
        "970406" => Some("DongABank"),
        "970409" => Some("BacABank"),
        "970410" => Some("StandardChartered"),
        "970414" => Some("Oceanbank"),
        "970421" => Some("VRB"),
        "970425" => Some("ABBANK"),
        "970427" => Some("VietABank"),
        "970431" => Some("Eximbank"),
        "970433" => Some("VietBank"),
        "970434" => Some("IndovinaBank"),
        "970438" => Some("BaoVietBank"),
        "970439" => Some("PublicBank"),
        "970443" => Some("SHB"),
        "970444" => Some("CBBank"),
        "970448" => Some("OCB"),
        "970452" => Some("KienLongBank"),
        "422589" => Some("CIMB"),
        "458761" => Some("HSBC"),
        "796500" => Some("DBSBank"),
        "801011" => Some("Nonghyup"),
        "970442" => Some("HongLeong"),
        "970457" => Some("Woori"),
        "970458" => Some("UnitedOverseas"),
        "970462" => Some("KookminHN"),
        "970463" => Some("KookminHCM"),
        "970446" => Some("COOPBANK"),
        _ => None,
    }
}

pub async fn generate_qr_transfer_image(
    bin: &str,
    account_number: &str,
    amount: Option<u64>,
    content: Option<&str>,
    template: &str,
) -> Result<PathBuf> {
    let sepay_bank = bin_to_sepay(bin).ok_or_else(|| {
        BizClawError::Channel(format!("Unsupported bank BIN for VietQR: {}", bin))
    })?;

    let mut url = format!(
        "https://qr.sepay.vn/img?bank={}&acc={}&template={}",
        sepay_bank, account_number, template
    );

    if let Some(amt) = amount {
        url.push_str(&format!("&amount={}", amt));
    }
    if let Some(desc) = content {
        let encoded_desc = urlencoding::encode(desc);
        url.push_str(&format!("&des={}", encoded_desc));
    }

    let client = reqwest::Client::new();
    let resp = client
        .get(&url)
        .header("User-Agent", "Mozilla/5.0")
        .send()
        .await
        .map_err(|e| BizClawError::Channel(format!("Failed to download QR: {}", e)))?;

    if !resp.status().is_success() {
        return Err(BizClawError::Channel(format!(
            "VietQR API returned error: {}",
            resp.status()
        )));
    }

    let bytes = resp
        .bytes()
        .await
        .map_err(|e| BizClawError::Channel(format!("Failed to read QR bytes: {}", e)))?;

    let tmp_path = std::env::temp_dir().join(format!("qr_{}_{}.png", bin, account_number));
    let mut file = std::fs::File::create(&tmp_path)
        .map_err(|e| BizClawError::Channel(format!("Failed to save QR file: {}", e)))?;
    file.write_all(&bytes)
        .map_err(|e| BizClawError::Channel(format!("Failed to save QR file: {}", e)))?;

    Ok(tmp_path)
}
