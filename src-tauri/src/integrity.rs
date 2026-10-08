use serde::{Deserialize, Serialize};
use std::{collections::BTreeMap, io::Read, path::Path};

#[derive(Clone, Default, Serialize, Deserialize)]
pub struct PageProof {
    pub url: String,
    pub file: String,
    pub size: u64,
    pub crc32: u32,
}
pub type PageProofs = BTreeMap<String, PageProof>;
#[derive(Serialize, Deserialize)]
pub struct ExportProof {
    pub version: u32,
    pub pages: usize,
    pub format: String,
    pub size: u64,
    pub crc32: u32,
}
pub fn image(bytes: &[u8]) -> Result<(), String> {
    if bytes.len() > 64 * 1024 * 1024 {
        return Err("Immagine oltre 64 MB".into());
    }
    let format = image::guess_format(bytes).map_err(|e| e.to_string())?;
    if format == image::ImageFormat::Jpeg && !bytes.ends_with(&[0xff, 0xd9]) {
        return Err("JPEG troncato: manca la fine dell’immagine".into());
    }
    let mut reader = image::ImageReader::with_format(std::io::Cursor::new(bytes), format);
    let mut limits = image::Limits::default();
    limits.max_image_width = Some(20000);
    limits.max_image_height = Some(20000);
    limits.max_alloc = Some(256 * 1024 * 1024);
    reader.limits(limits);
    reader
        .decode()
        .map_err(|e| format!("Immagine danneggiata o formato non supportato: {e}"))?;
    Ok(())
}
pub fn fingerprint(path: &Path) -> Result<(u64, u32), String> {
    let mut file = std::fs::File::open(path).map_err(|e| e.to_string())?;
    let mut hash = crc32fast::Hasher::new();
    let mut size = 0;
    let mut buffer = [0; 65536];
    loop {
        let n = file.read(&mut buffer).map_err(|e| e.to_string())?;
        if n == 0 {
            break;
        }
        size += n as u64;
        hash.update(&buffer[..n]);
    }
    Ok((size, hash.finalize()))
}
pub fn proof_path(output: &Path) -> std::path::PathBuf {
    let mut name = output.as_os_str().to_os_string();
    name.push(".integrity.json");
    name.into()
}
pub fn verify_export(
    output: &Path,
    format: &str,
    expected: Option<usize>,
) -> Result<usize, String> {
    let proof: ExportProof =
        serde_json::from_slice(&std::fs::read(proof_path(output)).map_err(|e| e.to_string())?)
            .map_err(|e| e.to_string())?;
    if proof.version != 1
        || proof.format != format
        || proof.pages == 0
        || expected.is_some_and(|n| n != proof.pages)
    {
        return Err("Conteggio pagine o formato non corrispondente".into());
    }
    if fingerprint(output)? != (proof.size, proof.crc32) {
        return Err("File esportato mancante o danneggiato".into());
    }
    Ok(proof.pages)
}
pub fn save_export_proof(output: &Path, format: &str, pages: usize) -> Result<(), String> {
    let (size, crc32) = fingerprint(output)?;
    let proof = ExportProof {
        version: 1,
        pages,
        format: format.into(),
        size,
        crc32,
    };
    let target = proof_path(output);
    let tmp = target.with_extension("json.part");
    std::fs::write(
        &tmp,
        serde_json::to_vec_pretty(&proof).map_err(|e| e.to_string())?,
    )
    .map_err(|e| e.to_string())?;
    std::fs::rename(tmp, target).map_err(|e| e.to_string())
}
