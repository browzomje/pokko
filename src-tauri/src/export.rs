use lopdf::{dictionary, Document, Object, Stream};
use std::path::{Path, PathBuf};

pub fn images(source: &Path) -> Result<Vec<PathBuf>, String> {
    let mut dirs = std::fs::read_dir(source)
        .map_err(|e| e.to_string())?
        .filter_map(Result::ok)
        .map(|e| e.path())
        .filter(|p| p.is_dir())
        .collect::<Vec<_>>();
    dirs.sort();
    let mut result = Vec::new();
    for dir in dirs {
        let mut files = std::fs::read_dir(dir)
            .map_err(|e| e.to_string())?
            .filter_map(Result::ok)
            .map(|e| e.path())
            .filter(|p| {
                p.extension().is_some_and(|e| {
                    ["jpg", "png", "webp", "gif", "avif"].contains(&e.to_string_lossy().as_ref())
                })
            })
            .collect::<Vec<_>>();
        files.sort();
        result.extend(files);
    }
    Ok(result)
}
pub fn pdf(
    source: &Path,
    destination: &Path,
    checkpoint: impl Fn() -> Result<(), String>,
) -> Result<(), String> {
    let mut doc = Document::with_version("1.5");
    let root = doc.new_object_id();
    let mut kids = Vec::new();
    for path in images(source)? {
        checkpoint()?;
        let mut reader = image::ImageReader::open(&path)
            .map_err(|e| e.to_string())?
            .with_guessed_format()
            .map_err(|e| e.to_string())?;
        let mut limits = image::Limits::default();
        limits.max_image_width = Some(20000);
        limits.max_image_height = Some(20000);
        limits.max_alloc = Some(256 * 1024 * 1024);
        reader.limits(limits);
        let img = reader
            .decode()
            .map_err(|e| format!("Conversione PDF di {}: {e}", path.display()))?
            .to_rgb8();
        let (w, h) = img.dimensions();
        let mut bytes = Vec::new();
        image::codecs::jpeg::JpegEncoder::new_with_quality(&mut bytes, 95)
            .encode_image(&img)
            .map_err(|e| e.to_string())?;
        let image=doc.add_object(Stream::new(dictionary!{"Type"=>"XObject","Subtype"=>"Image","Width"=>w,"Height"=>h,"ColorSpace"=>"DeviceRGB","BitsPerComponent"=>8,"Filter"=>"DCTDecode"},bytes));
        let content = doc.add_object(Stream::new(
            dictionary! {},
            format!("q {w} 0 0 {h} 0 0 cm /Scan Do Q").into_bytes(),
        ));
        let page=doc.add_object(dictionary!{"Type"=>"Page","Parent"=>root,"MediaBox"=>vec![0.into(),0.into(),w.into(),h.into()],"Resources"=>dictionary!{"XObject"=>dictionary!{"Scan"=>image}},"Contents"=>content});
        kids.push(Object::Reference(page));
    }
    if kids.is_empty() {
        return Err("Nessuna pagina da esportare".into());
    }
    doc.objects.insert(
        root,
        dictionary! {"Type"=>"Pages","Count"=>kids.len() as i64,"Kids"=>kids}.into(),
    );
    let catalog = doc.add_object(dictionary! {"Type"=>"Catalog","Pages"=>root});
    doc.trailer.set("Root", catalog);
    checkpoint()?;
    let tmp = destination.with_extension("pdf.part");
    doc.save(&tmp).map_err(|e| e.to_string())?;
    std::fs::rename(tmp, destination).map_err(|e| e.to_string())
}
