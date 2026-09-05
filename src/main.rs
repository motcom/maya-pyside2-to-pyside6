use std::{
    env, fs,
    path::{Path, PathBuf},
};

use encoding_rs::SHIFT_JIS;
use walkdir::WalkDir;

enum ReplaceResult {
    Changed,
    Unchanged,
    Skipped,
}

enum Encoding {
    Utf8,
    Utf8Bom,
    Utf16Le,
    Utf16Be,
    ShiftJis,
}

fn main() {
    let version = env::args()
        .nth(1)
        .expect("第一引数にバージョンを入力してください");

    let root = default_maya_path(version);

    if !root.exists() {
        eprintln!("ディレクトリが存在しません: {}", root.display());
        return;
    }

    println!("検索開始: {}", root.display());
    println!("置換内容: PySide2 -> PySide6");
    println!();

    let mut scanned = 0;
    let mut changed = 0;
    let mut skipped = 0;

    for entry in WalkDir::new(&root)
        .into_iter()
        .filter_map(Result::ok)
        .filter(|entry| entry.file_type().is_file())
    {
        let path = entry.path();

        scanned += 1;

        match replace_pyside2(path) {
            ReplaceResult::Changed => {
                changed += 1;
                println!("変更: {}", path.display());
            }

            ReplaceResult::Unchanged => {}

            ReplaceResult::Skipped => {
                skipped += 1;
            }
        }
    }

    println!();
    println!("完了");
    println!("走査したファイル:     {scanned}");
    println!("変更したファイル:     {changed}");
    println!("読み飛ばしたファイル: {skipped}");
}

fn replace_pyside2(path: &Path) -> ReplaceResult {
    let Ok(bytes) = fs::read(path) else {
        eprintln!("読み込み失敗: {}", path.display());
        return ReplaceResult::Skipped;
    };

    // バイナリっぽいファイルを簡易判定
    if is_binary(&bytes) {
        return ReplaceResult::Skipped;
    }

    let Some((text, encoding)) = decode_text(&bytes) else {
        return ReplaceResult::Skipped;
    };

    if !text.contains("PySide2") {
        return ReplaceResult::Unchanged;
    }

    let new_text = text.replace("PySide2", "PySide6");

    let new_bytes = encode_text(&new_text, encoding);

    if let Err(err) = fs::write(path, new_bytes) {
        eprintln!("書き込み失敗: {}: {err}", path.display());
        return ReplaceResult::Skipped;
    }

    ReplaceResult::Changed
}

fn decode_text(bytes: &[u8]) -> Option<(String, Encoding)> {
    // UTF-8 BOM
    if bytes.starts_with(&[0xEF, 0xBB, 0xBF]) {
        let text = String::from_utf8(bytes[3..].to_vec()).ok()?;
        return Some((text, Encoding::Utf8Bom));
    }

    // UTF-16 LE BOM
    if bytes.starts_with(&[0xFF, 0xFE]) {
        let data = &bytes[2..];

        let utf16: Vec<u16> = data
            .chunks_exact(2)
            .map(|c| u16::from_le_bytes([c[0], c[1]]))
            .collect();

        let text = String::from_utf16(&utf16).ok()?;

        return Some((text, Encoding::Utf16Le));
    }

    // UTF-16 BE BOM
    if bytes.starts_with(&[0xFE, 0xFF]) {
        let data = &bytes[2..];

        let utf16: Vec<u16> = data
            .chunks_exact(2)
            .map(|c| u16::from_be_bytes([c[0], c[1]]))
            .collect();

        let text = String::from_utf16(&utf16).ok()?;

        return Some((text, Encoding::Utf16Be));
    }

    // 通常の UTF-8
    if let Ok(text) = String::from_utf8(bytes.to_vec()) {
        return Some((text, Encoding::Utf8));
    }

    // Shift_JIS / Windows-31J
    let (text, _, had_errors) = SHIFT_JIS.decode(bytes);

    if !had_errors {
        return Some((text.into_owned(), Encoding::ShiftJis));
    }

    None
}

fn encode_text(text: &str, encoding: Encoding) -> Vec<u8> {
    match encoding {
        Encoding::Utf8 => text.as_bytes().to_vec(),

        Encoding::Utf8Bom => {
            let mut bytes = vec![0xEF, 0xBB, 0xBF];
            bytes.extend_from_slice(text.as_bytes());
            bytes
        }

        Encoding::Utf16Le => {
            let mut bytes = vec![0xFF, 0xFE];

            for unit in text.encode_utf16() {
                bytes.extend_from_slice(&unit.to_le_bytes());
            }

            bytes
        }

        Encoding::Utf16Be => {
            let mut bytes = vec![0xFE, 0xFF];

            for unit in text.encode_utf16() {
                bytes.extend_from_slice(&unit.to_be_bytes());
            }

            bytes
        }

        Encoding::ShiftJis => {
            let (bytes, _, _) = SHIFT_JIS.encode(text);
            bytes.into_owned()
        }
    }
}

fn is_binary(bytes: &[u8]) -> bool {
    if bytes.is_empty() {
        return false;
    }

    // UTF-16 BOM がある場合はテキスト
    if bytes.starts_with(&[0xFF, 0xFE]) || bytes.starts_with(&[0xFE, 0xFF]) {
        return false;
    }

    // NULL が多ければバイナリと判断
    let null_count = bytes.iter().filter(|&&b| b == 0).count();

    null_count > bytes.len() / 10
}

fn default_maya_path(version: String) -> PathBuf {
    let user_profile = env::var("USERPROFILE").unwrap_or_else(|_| ".".to_string());

    PathBuf::from(user_profile)
        .join("Documents")
        .join("maya")
        .join(version)
}
