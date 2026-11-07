//! Conversão de uma imagem em um PDF com uma página por quadro.
//!
//! A página é A4, retrato ou paisagem conforme a proporção da imagem. O desenho fica
//! centralizado e é reduzido sem distorcer; com a opção de DPI, o tamanho físico gravado no
//! arquivo é respeitado enquanto couber na folha. Imagens com transparência são compostas
//! sobre branco e todo quadro vira uma imagem RGB de 8 bits comprimida com zlib.

use flate2::write::ZlibEncoder;
use flate2::Compression;
use image::{DynamicImage, ImageFormat, ImageReader};
use lopdf::{dictionary, Document, Object, ObjectId, Stream};
use std::io::{Cursor, Write};
use tiff::decoder::{Decoder, DecodingResult, Limits};
use tiff::tags::Tag;
use tiff::ColorType as CorDoTiff;

/// Maior arquivo de imagem aceito (1 GiB).
pub const LIMITE_BYTES_IMAGEM: u64 = 1024 * 1024 * 1024;

/// Maior número de pixels de um quadro (100 megapixels).
pub const LIMITE_PIXELS: u64 = 100_000_000;

/// Maior largura ou altura, em pixels, de um quadro.
pub const LIMITE_DIMENSAO: u32 = 100_000;

/// Maior número de quadros lido de um TIFF.
pub const LIMITE_QUADROS: usize = 10_000;

/// Teto de memória de uma decodificação. Cobre o pior caso de um quadro dentro de
/// [`LIMITE_PIXELS`] que ainda é convertido: RGB de 16 bits ocupa 6 bytes por pixel, ou 573 MiB
/// nos 100 MP. Um quadro com alfa de 16 bits no mesmo limite precisa de mais que isso e é recusado
/// como falha de decodificação, nunca aceito acima do limite de pixels.
const TETO_DE_ALOCACAO: u64 = 768 << 20;

/// Largura e altura da folha A4, em pontos (1/72 de polegada).
const A4_RETRATO: (f64, f64) = (595.0, 842.0);

/// Opções da conversão. O painel escolhe se o DPI do arquivo define o tamanho físico.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct OpcoesDeImagem {
    pub usar_dpi: bool,
}

impl Default for OpcoesDeImagem {
    fn default() -> Self {
        Self { usar_dpi: false }
    }
}

/// Motivo de a imagem não virar PDF. A interface traduz pelo código, não pela mensagem.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ErroImagem {
    /// Nenhum byte para decodificar.
    Vazio,
    /// O arquivo passa do limite de 1 GiB.
    Grande,
    /// Um quadro passa do limite de 100 megapixels ou de 100 mil pixels em um eixo.
    QuadroGrande,
    /// Extensão sem conversão definida. Bytes de outro formato com extensão aceita caem em
    /// [`ErroImagem::Decodificacao`].
    FormatoNaoSuportado,
    /// O decodificador recusou o conteúdo.
    Decodificacao(String),
    /// O arquivo não tem nenhum quadro legível.
    SemQuadros,
    /// O TIFF passa do limite de quadros.
    QuadrosDemais,
    /// Falha ao montar os bytes do PDF.
    Escrita(String),
}

impl std::fmt::Display for ErroImagem {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Vazio => f.write_str("a imagem está vazia"),
            Self::Grande => f.write_str("a imagem passa do limite de 1 GiB"),
            Self::QuadroGrande => {
                f.write_str("um quadro da imagem passa do limite de 100 megapixels")
            }
            Self::FormatoNaoSuportado => f.write_str("formato de imagem não suportado"),
            Self::Decodificacao(detalhe) => write!(f, "não foi possível ler a imagem: {detalhe}"),
            Self::SemQuadros => f.write_str("a imagem não tem quadros legíveis"),
            Self::QuadrosDemais => f.write_str("o TIFF passa do limite de quadros"),
            Self::Escrita(detalhe) => write!(f, "não foi possível montar o PDF: {detalhe}"),
        }
    }
}

impl std::error::Error for ErroImagem {}

/// Um quadro já decodificado, com a medida física opcional que veio do arquivo.
struct Quadro {
    largura: u32,
    altura: u32,
    rgb: Vec<u8>,
    dpi: Option<(f64, f64)>,
}

/// Monta o PDF aos poucos: cada quadro é comprimido e entra no documento antes de o próximo
/// ser decodificado, para um TIFF multipágina não manter todos os pixels na memória.
struct Montagem {
    documento: Document,
    raiz: ObjectId,
    paginas: Vec<Object>,
    opcoes: OpcoesDeImagem,
}

impl Montagem {
    fn nova(opcoes: OpcoesDeImagem) -> Self {
        let mut documento = Document::with_version("1.5");
        let raiz = documento.new_object_id();
        Self {
            documento,
            raiz,
            paginas: Vec::new(),
            opcoes,
        }
    }

    fn adicionar(&mut self, quadro: Quadro) -> Result<(), ErroImagem> {
        let (pagina_largura, pagina_altura) = if quadro.largura > quadro.altura {
            (A4_RETRATO.1, A4_RETRATO.0)
        } else {
            A4_RETRATO
        };
        let (desenho_largura, desenho_altura) =
            tamanho_do_desenho(&quadro, pagina_largura, pagina_altura, self.opcoes.usar_dpi);
        if !desenho_largura.is_finite()
            || !desenho_altura.is_finite()
            || desenho_largura <= 0.0
            || desenho_altura <= 0.0
        {
            return Err(ErroImagem::Decodificacao(
                "medida da imagem inválida".to_owned(),
            ));
        }
        let deslocamento_x = (pagina_largura - desenho_largura) / 2.0;
        let deslocamento_y = (pagina_altura - desenho_altura) / 2.0;
        let conteudo = format!(
            "q {desenho_largura:.4} 0 0 {desenho_altura:.4} {deslocamento_x:.4} {deslocamento_y:.4} cm /Imagem Do Q"
        );
        let conteudo_id = self
            .documento
            .add_object(Stream::new(dictionary! {}, conteudo.into_bytes()));
        let comprimido = comprimir(&quadro.rgb)?;
        let imagem = self.documento.add_object(Stream::new(
            dictionary! {
                "Type" => "XObject",
                "Subtype" => "Image",
                "Width" => i64::from(quadro.largura),
                "Height" => i64::from(quadro.altura),
                "ColorSpace" => "DeviceRGB",
                "BitsPerComponent" => 8,
                "Filter" => "FlateDecode",
            },
            comprimido,
        ));
        let pagina = self.documento.add_object(dictionary! {
            "Type" => "Page",
            "Parent" => self.raiz,
            "MediaBox" => vec![
                Object::Real(0.0),
                Object::Real(0.0),
                Object::Real(pagina_largura as f32),
                Object::Real(pagina_altura as f32),
            ],
            "Resources" => dictionary! { "XObject" => dictionary! { "Imagem" => imagem } },
            "Contents" => conteudo_id,
        });
        self.paginas.push(Object::Reference(pagina));
        Ok(())
    }

    fn terminar(mut self) -> Result<Vec<u8>, ErroImagem> {
        if self.paginas.is_empty() {
            return Err(ErroImagem::SemQuadros);
        }
        let total = self.paginas.len();
        self.documento.objects.insert(
            self.raiz,
            Object::Dictionary(dictionary! {
                "Type" => "Pages",
                "Count" => total as i64,
                "Kids" => self.paginas,
            }),
        );
        let catalogo = self
            .documento
            .add_object(dictionary! { "Type" => "Catalog", "Pages" => self.raiz });
        self.documento.trailer.set("Root", catalogo);
        let mut bytes = Vec::new();
        self.documento
            .save_to(&mut bytes)
            .map_err(|falha| ErroImagem::Escrita(falha.to_string()))?;
        Ok(bytes)
    }
}

/// Converte a imagem em PDF. `extensao` é a do arquivo original, sem ponto.
pub fn pdf_de_imagem(
    bytes: &[u8],
    extensao: &str,
    opcoes: &OpcoesDeImagem,
) -> Result<Vec<u8>, ErroImagem> {
    conferir_tamanho(bytes.len() as u64)?;
    let extensao = extensao.trim_start_matches('.').to_ascii_lowercase();
    match extensao.as_str() {
        "tif" | "tiff" => pdf_de_tiff(bytes, *opcoes),
        "png" => pdf_de_raster(bytes, ImageFormat::Png, dpi_do_png(bytes), *opcoes),
        "jpg" | "jpeg" => pdf_de_raster(bytes, ImageFormat::Jpeg, dpi_do_jpeg(bytes), *opcoes),
        "bmp" => pdf_de_raster(bytes, ImageFormat::Bmp, dpi_do_bmp(bytes), *opcoes),
        "gif" => pdf_de_raster(bytes, ImageFormat::Gif, None, *opcoes),
        "webp" => pdf_de_raster(bytes, ImageFormat::WebP, None, *opcoes),
        _ => Err(ErroImagem::FormatoNaoSuportado),
    }
}

fn conferir_tamanho(tamanho: u64) -> Result<(), ErroImagem> {
    if tamanho == 0 {
        return Err(ErroImagem::Vazio);
    }
    if tamanho > LIMITE_BYTES_IMAGEM {
        return Err(ErroImagem::Grande);
    }
    Ok(())
}

fn conferir_dimensoes(largura: u32, altura: u32) -> Result<(), ErroImagem> {
    if largura == 0 || altura == 0 {
        return Err(ErroImagem::Vazio);
    }
    if largura > LIMITE_DIMENSAO || altura > LIMITE_DIMENSAO {
        return Err(ErroImagem::QuadroGrande);
    }
    let pixels = u64::from(largura)
        .checked_mul(u64::from(altura))
        .ok_or(ErroImagem::QuadroGrande)?;
    if pixels > LIMITE_PIXELS {
        return Err(ErroImagem::QuadroGrande);
    }
    Ok(())
}

fn pdf_de_raster(
    bytes: &[u8],
    formato: ImageFormat,
    dpi: Option<(f64, f64)>,
    opcoes: OpcoesDeImagem,
) -> Result<Vec<u8>, ErroImagem> {
    let dimensoes = ImageReader::with_format(Cursor::new(bytes), formato)
        .into_dimensions()
        .map_err(|falha| ErroImagem::Decodificacao(falha.to_string()))?;
    conferir_dimensoes(dimensoes.0, dimensoes.1)?;
    let mut leitor = ImageReader::with_format(Cursor::new(bytes), formato);
    let mut limites = image::Limits::default();
    limites.max_image_width = Some(LIMITE_DIMENSAO);
    limites.max_image_height = Some(LIMITE_DIMENSAO);
    limites.max_alloc = Some(TETO_DE_ALOCACAO);
    leitor.limits(limites);
    let imagem = leitor
        .decode()
        .map_err(|falha| ErroImagem::Decodificacao(falha.to_string()))?;
    let quadro = Quadro {
        largura: dimensoes.0,
        altura: dimensoes.1,
        rgb: rgb_da_imagem(imagem, dimensoes.0, dimensoes.1)?,
        dpi: dpi_valido(dpi),
    };
    let mut montagem = Montagem::nova(opcoes);
    montagem.adicionar(quadro)?;
    montagem.terminar()
}

fn rgb_da_imagem(imagem: DynamicImage, largura: u32, altura: u32) -> Result<Vec<u8>, ErroImagem> {
    let esperado = usize::try_from(u64::from(largura) * u64::from(altura))
        .map_err(|_| ErroImagem::QuadroGrande)?;
    let rgb = if imagem.color().has_alpha() {
        let rgba = imagem.into_rgba8();
        if rgba.len() != esperado.saturating_mul(4) {
            return Err(ErroImagem::Decodificacao(
                "quantidade de pixels diferente da medida do quadro".to_owned(),
            ));
        }
        compor_sobre_branco(rgba.as_raw())
    } else {
        let rgb = imagem.into_rgb8();
        if rgb.len() != esperado.saturating_mul(3) {
            return Err(ErroImagem::Decodificacao(
                "quantidade de pixels diferente da medida do quadro".to_owned(),
            ));
        }
        rgb.into_raw()
    };
    Ok(rgb)
}

/// Mistura RGBA sobre branco: `(canal * alfa + 255 * (255 - alfa)) / 255`.
fn compor_sobre_branco(rgba: &[u8]) -> Vec<u8> {
    let mut rgb = Vec::with_capacity(rgba.len() / 4 * 3);
    for pixel in rgba.chunks_exact(4) {
        let alfa = u32::from(pixel[3]);
        for canal in &pixel[..3] {
            let misturado = (u32::from(*canal) * alfa + 255 * (255 - alfa) + 127) / 255;
            rgb.push(u8::try_from(misturado).unwrap_or(255));
        }
    }
    rgb
}

fn comprimir(dados: &[u8]) -> Result<Vec<u8>, ErroImagem> {
    let mut escritor = ZlibEncoder::new(Vec::new(), Compression::default());
    escritor
        .write_all(dados)
        .map_err(|falha| ErroImagem::Escrita(falha.to_string()))?;
    escritor
        .finish()
        .map_err(|falha| ErroImagem::Escrita(falha.to_string()))
}

fn pdf_de_tiff(bytes: &[u8], opcoes: OpcoesDeImagem) -> Result<Vec<u8>, ErroImagem> {
    let mut limites = Limits::default();
    limites.decoding_buffer_size = usize::try_from(TETO_DE_ALOCACAO).unwrap_or(usize::MAX);
    limites.ifd_value_size = 16 << 20;
    let mut decodificador = Decoder::new(Cursor::new(bytes))
        .map_err(|falha| ErroImagem::Decodificacao(falha.to_string()))?
        .with_limits(limites);
    let mut montagem = Montagem::nova(opcoes);
    let mut quadros = 0usize;
    loop {
        if quadros >= LIMITE_QUADROS {
            return Err(ErroImagem::QuadrosDemais);
        }
        let (largura, altura) = decodificador
            .dimensions()
            .map_err(|falha| ErroImagem::Decodificacao(falha.to_string()))?;
        conferir_dimensoes(largura, altura)?;
        let dpi = dpi_do_tiff(&mut decodificador).and_then(|dpi| dpi_valido(Some(dpi)));
        let cor = decodificador
            .colortype()
            .map_err(|falha| ErroImagem::Decodificacao(falha.to_string()))?;
        let mapa = if matches!(cor, CorDoTiff::Palette(_)) {
            Some(
                decodificador
                    .get_tag_u16_vec(Tag::ColorMap)
                    .map_err(|falha| ErroImagem::Decodificacao(falha.to_string()))?,
            )
        } else {
            None
        };
        let dados = decodificador
            .read_image()
            .map_err(|falha| ErroImagem::Decodificacao(falha.to_string()))?;
        let rgb = rgb_do_tiff(cor, dados, mapa.as_deref(), largura, altura)?;
        montagem.adicionar(Quadro {
            largura,
            altura,
            rgb,
            dpi,
        })?;
        quadros += 1;
        if !decodificador.more_images() {
            break;
        }
        decodificador
            .next_image()
            .map_err(|falha| ErroImagem::Decodificacao(falha.to_string()))?;
    }
    montagem.terminar()
}

fn rgb_do_tiff(
    cor: CorDoTiff,
    dados: DecodingResult,
    mapa: Option<&[u16]>,
    largura: u32,
    altura: u32,
) -> Result<Vec<u8>, ErroImagem> {
    let pixels = usize::try_from(u64::from(largura) * u64::from(altura))
        .map_err(|_| ErroImagem::QuadroGrande)?;
    match (cor, dados) {
        (CorDoTiff::RGB(8), DecodingResult::U8(dados)) => {
            conferir_bytes(&dados, pixels.checked_mul(3), "RGB de 8 bits")?;
            Ok(dados)
        }
        (CorDoTiff::RGB(16), DecodingResult::U16(dados)) => {
            conferir_bytes(&dados, pixels.checked_mul(3), "RGB de 16 bits")?;
            Ok(dados.iter().map(|v| (v >> 8) as u8).collect())
        }
        (CorDoTiff::RGBA(8), DecodingResult::U8(dados)) => {
            conferir_bytes(&dados, pixels.checked_mul(4), "RGBA de 8 bits")?;
            Ok(compor_sobre_branco(&dados))
        }
        (CorDoTiff::RGBA(16), DecodingResult::U16(dados)) => {
            conferir_bytes(&dados, pixels.checked_mul(4), "RGBA de 16 bits")?;
            let estreitados: Vec<u8> = dados.iter().map(|v| (v >> 8) as u8).collect();
            Ok(compor_sobre_branco(&estreitados))
        }
        (CorDoTiff::Gray(8), DecodingResult::U8(dados)) => {
            conferir_bytes(&dados, Some(pixels), "cinza de 8 bits")?;
            Ok(cinza_para_rgb(&dados))
        }
        (CorDoTiff::Gray(16), DecodingResult::U16(dados)) => {
            conferir_bytes(&dados, Some(pixels), "cinza de 16 bits")?;
            let estreitados: Vec<u8> = dados.iter().map(|v| (v >> 8) as u8).collect();
            Ok(cinza_para_rgb(&estreitados))
        }
        (CorDoTiff::GrayA(8), DecodingResult::U8(dados)) => {
            conferir_bytes(&dados, pixels.checked_mul(2), "cinza com alfa de 8 bits")?;
            let mut rgb = Vec::with_capacity(pixels * 3);
            for pixel in dados.chunks_exact(2) {
                let cinza = u32::from(pixel[0]);
                let alfa = u32::from(pixel[1]);
                let misturado = (cinza * alfa + 255 * (255 - alfa) + 127) / 255;
                let valor = u8::try_from(misturado).unwrap_or(255);
                rgb.extend([valor, valor, valor]);
            }
            Ok(rgb)
        }
        (CorDoTiff::CMYK(8), DecodingResult::U8(dados)) => {
            conferir_bytes(&dados, pixels.checked_mul(4), "CMYK de 8 bits")?;
            Ok(cmyk_para_rgb(&dados))
        }
        (CorDoTiff::CMYKA(8), DecodingResult::U8(dados)) => {
            conferir_bytes(&dados, pixels.checked_mul(5), "CMYK com alfa de 8 bits")?;
            let mut rgb = Vec::with_capacity(pixels * 3);
            for pixel in dados.chunks_exact(5) {
                let cmyk = cmyk_para_rgb(&pixel[..4]);
                let alfa = u32::from(pixel[4]);
                for canal in cmyk {
                    let misturado = (u32::from(canal) * alfa + 255 * (255 - alfa) + 127) / 255;
                    rgb.push(u8::try_from(misturado).unwrap_or(255));
                }
            }
            Ok(rgb)
        }
        (CorDoTiff::Palette(8), DecodingResult::U8(dados)) => {
            conferir_bytes(&dados, Some(pixels), "paleta de 8 bits")?;
            let mapa = mapa.ok_or_else(|| {
                ErroImagem::Decodificacao("paleta do TIFF sem tabela de cores".to_owned())
            })?;
            let tons = 256usize;
            if mapa.len() < tons * 3 {
                return Err(ErroImagem::Decodificacao(
                    "tabela de cores do TIFF incompleta".to_owned(),
                ));
            }
            let mut rgb = Vec::with_capacity(pixels * 3);
            for indice in dados {
                let indice = usize::from(indice);
                for canal in 0..3 {
                    rgb.push((mapa[indice + canal * tons] >> 8) as u8);
                }
            }
            Ok(rgb)
        }
        (outra, _) => Err(ErroImagem::Decodificacao(format!(
            "tipo de cor do TIFF não suportado: {outra:?}"
        ))),
    }
}

fn conferir_bytes<T>(
    dados: &[T],
    esperado: Option<usize>,
    descricao: &str,
) -> Result<(), ErroImagem> {
    if Some(dados.len()) != esperado {
        return Err(ErroImagem::Decodificacao(format!(
            "quantidade de bytes diferente da medida do quadro ({descricao})"
        )));
    }
    Ok(())
}

fn cinza_para_rgb(dados: &[u8]) -> Vec<u8> {
    let mut rgb = Vec::with_capacity(dados.len() * 3);
    for valor in dados {
        rgb.extend([*valor, *valor, *valor]);
    }
    rgb
}

fn cmyk_para_rgb(dados: &[u8]) -> Vec<u8> {
    let mut rgb = Vec::with_capacity(dados.len() / 4 * 3);
    for pixel in dados.chunks_exact(4) {
        let c = u32::from(pixel[0]);
        let m = u32::from(pixel[1]);
        let y = u32::from(pixel[2]);
        let k = u32::from(pixel[3]);
        for canal in [c, m, y] {
            let valor = ((255 - canal) * (255 - k) + 127) / 255;
            rgb.push(u8::try_from(valor).unwrap_or(0));
        }
    }
    rgb
}

/// Medida do desenho na folha, em pontos. Sem DPI, a imagem ocupa a maior área possível da
/// A4. Com DPI, usa a medida física do arquivo enquanto ela couber na folha.
fn tamanho_do_desenho(
    quadro: &Quadro,
    pagina_largura: f64,
    pagina_altura: f64,
    usar_dpi: bool,
) -> (f64, f64) {
    let largura_px = f64::from(quadro.largura);
    let altura_px = f64::from(quadro.altura);
    if usar_dpi {
        if let Some((dpi_x, dpi_y)) = quadro.dpi {
            let natural_largura = largura_px * 72.0 / dpi_x;
            let natural_altura = altura_px * 72.0 / dpi_y;
            let fator = 1.0f64
                .min(pagina_largura / natural_largura)
                .min(pagina_altura / natural_altura);
            return (natural_largura * fator, natural_altura * fator);
        }
    }
    let fator = (pagina_largura / largura_px).min(pagina_altura / altura_px);
    (largura_px * fator, altura_px * fator)
}

/// DPI do TIFF: resolução x/y do primeiro quadro, convertida pela unidade declarada.
fn dpi_do_tiff<R: std::io::Read + std::io::Seek>(
    decodificador: &mut Decoder<R>,
) -> Option<(f64, f64)> {
    let x = decodificador.get_tag_f64(Tag::XResolution).ok()?;
    let y = decodificador.get_tag_f64(Tag::YResolution).ok()?;
    let unidade = decodificador
        .get_tag_unsigned::<u16>(Tag::ResolutionUnit)
        .unwrap_or(2);
    let fator = match unidade {
        2 => 1.0,
        3 => 2.54,
        _ => return None,
    };
    Some((x * fator, y * fator))
}

fn dpi_valido(dpi: Option<(f64, f64)>) -> Option<(f64, f64)> {
    let (x, y) = dpi?;
    let plausivel = |valor: f64| valor.is_finite() && (1.0..=10_000.0).contains(&valor);
    (plausivel(x) && plausivel(y)).then_some((x, y))
}

/// DPI do PNG: bloco `pHYs`, com pixels por metro.
fn dpi_do_png(bytes: &[u8]) -> Option<(f64, f64)> {
    const ASSINATURA: [u8; 8] = [137, 80, 78, 71, 13, 10, 26, 10];
    if bytes.get(..8)? != ASSINATURA {
        return None;
    }
    let mut posicao = 8usize;
    while posicao.checked_add(8)? <= bytes.len() {
        let tamanho =
            u32::from_be_bytes(bytes.get(posicao..posicao + 4)?.try_into().ok()?) as usize;
        let tipo = bytes.get(posicao + 4..posicao + 8)?;
        let inicio = posicao.checked_add(8)?;
        let fim = inicio.checked_add(tamanho)?;
        if fim > bytes.len() {
            return None;
        }
        if tipo == b"pHYs" && tamanho == 9 {
            let x = u32::from_be_bytes(bytes.get(inicio..inicio + 4)?.try_into().ok()?);
            let y = u32::from_be_bytes(bytes.get(inicio + 4..inicio + 8)?.try_into().ok()?);
            let unidade = *bytes.get(inicio + 8)?;
            if unidade == 1 {
                return Some((f64::from(x) * 0.0254, f64::from(y) * 0.0254));
            }
            return None;
        }
        if tipo == b"IDAT" || tipo == b"IEND" {
            return None;
        }
        posicao = fim.checked_add(4)?;
    }
    None
}

/// DPI do JPEG: segmento APP0 JFIF.
fn dpi_do_jpeg(bytes: &[u8]) -> Option<(f64, f64)> {
    if bytes.get(..2)? != [0xFF, 0xD8] {
        return None;
    }
    let mut posicao = 2usize;
    while posicao.checked_add(4)? <= bytes.len() {
        if *bytes.get(posicao)? != 0xFF {
            return None;
        }
        let marcador = *bytes.get(posicao + 1)?;
        if marcador == 0xD8 || marcador == 0x01 || (0xD0..=0xD7).contains(&marcador) {
            posicao += 2;
            continue;
        }
        if marcador == 0xDA || marcador == 0xD9 {
            return None;
        }
        let tamanho = usize::from(u16::from_be_bytes(
            bytes.get(posicao + 2..posicao + 4)?.try_into().ok()?,
        ));
        if tamanho < 2 {
            return None;
        }
        let fim = posicao.checked_add(2)?.checked_add(tamanho)?;
        if fim > bytes.len() {
            return None;
        }
        if marcador == 0xE0 {
            let dados = bytes.get(posicao + 4..fim)?;
            if dados.get(..5)? == b"JFIF\0" && dados.len() >= 12 {
                let unidade = dados[7];
                let x = u16::from_be_bytes([dados[8], dados[9]]);
                let y = u16::from_be_bytes([dados[10], dados[11]]);
                let dpi = match unidade {
                    1 => Some((f64::from(x), f64::from(y))),
                    2 => Some((f64::from(x) * 2.54, f64::from(y) * 2.54)),
                    _ => None,
                };
                if dpi.is_some() {
                    return dpi;
                }
            }
        }
        posicao = fim;
    }
    None
}

/// DPI do BMP: pixels por metro do cabeçalho de informação.
fn dpi_do_bmp(bytes: &[u8]) -> Option<(f64, f64)> {
    if bytes.get(..2)? != b"BM" {
        return None;
    }
    let cabecalho = u32::from_le_bytes(bytes.get(14..18)?.try_into().ok()?);
    if cabecalho < 40 {
        return None;
    }
    let x = i32::from_le_bytes(bytes.get(38..42)?.try_into().ok()?);
    let y = i32::from_le_bytes(bytes.get(42..46)?.try_into().ok()?);
    if x <= 0 || y <= 0 {
        return None;
    }
    Some((f64::from(x) * 0.0254, f64::from(y) * 0.0254))
}

#[cfg(test)]
mod testes {
    use super::*;
    use crate::PdfOrigem;
    use image::{Rgb, RgbImage, Rgba, RgbaImage};

    fn rgb_de_teste(largura: u32, altura: u32) -> RgbImage {
        RgbImage::from_fn(largura, altura, |x, _| {
            Rgb([(x % 256) as u8, ((x / 2) % 256) as u8, 40])
        })
    }

    fn bytes_de(imagem: &DynamicImage, formato: ImageFormat) -> Vec<u8> {
        let mut bytes = Cursor::new(Vec::new());
        imagem
            .write_to(&mut bytes, formato)
            .expect("codificar imagem");
        bytes.into_inner()
    }

    fn documento(bytes: &[u8]) -> Document {
        Document::load_mem(bytes).expect("pdf derivado válido")
    }

    fn paginas(doc: &Document) -> Vec<ObjectId> {
        doc.get_pages().into_values().collect()
    }

    fn caixa(doc: &Document, pagina: ObjectId) -> (f32, f32) {
        let dict = doc.get_object(pagina).unwrap().as_dict().unwrap();
        let caixa = dict.get(b"MediaBox").unwrap().as_array().unwrap();
        (caixa[2].as_float().unwrap(), caixa[3].as_float().unwrap())
    }

    fn imagem_da_pagina(doc: &Document, pagina: ObjectId) -> (u32, u32, Vec<u8>) {
        let dict = doc.get_object(pagina).unwrap().as_dict().unwrap();
        let recursos = dict.get(b"Resources").unwrap().as_dict().unwrap();
        let xobjetos = recursos.get(b"XObject").unwrap().as_dict().unwrap();
        let id = xobjetos.get(b"Imagem").unwrap().as_reference().unwrap();
        let stream = doc.get_object(id).unwrap().as_stream().unwrap();
        let largura = stream.dict.get(b"Width").unwrap().as_i64().unwrap() as u32;
        let altura = stream.dict.get(b"Height").unwrap().as_i64().unwrap() as u32;
        let pixels = stream.decompressed_content().unwrap();
        (largura, altura, pixels)
    }

    fn conteudo(doc: &Document, pagina: ObjectId) -> String {
        String::from_utf8(doc.get_page_content(pagina)).unwrap()
    }

    #[test]
    fn png_vira_pdf_de_uma_pagina_paisagem() {
        let imagem = DynamicImage::ImageRgb8(rgb_de_teste(800, 600));
        let bytes = bytes_de(&imagem, ImageFormat::Png);
        let pdf = pdf_de_imagem(&bytes, "png", &OpcoesDeImagem::default()).unwrap();
        let doc = documento(&pdf);
        let paginas = paginas(&doc);
        assert_eq!(paginas.len(), 1);
        assert_eq!(caixa(&doc, paginas[0]), (842.0, 595.0));
        let (largura, altura, pixels) = imagem_da_pagina(&doc, paginas[0]);
        assert_eq!((largura, altura), (800, 600));
        assert_eq!(
            pixels.as_slice(),
            imagem.as_rgb8().unwrap().as_raw().as_slice()
        );
    }

    #[test]
    fn cada_formato_aceito_vira_pdf_com_os_pixels_certos() {
        let origem = rgb_de_teste(23, 11);
        let imagem = DynamicImage::ImageRgb8(origem.clone());
        let casos = [
            (ImageFormat::Png, "png"),
            (ImageFormat::Jpeg, "jpg"),
            (ImageFormat::Bmp, "bmp"),
            (ImageFormat::Gif, "gif"),
            (ImageFormat::WebP, "webp"),
        ];
        for (formato, extensao) in casos {
            let bytes = bytes_de(&imagem, formato);
            let pdf = pdf_de_imagem(&bytes, extensao, &OpcoesDeImagem::default())
                .unwrap_or_else(|falha| panic!("{extensao}: {falha}"));
            let doc = documento(&pdf);
            let paginas = paginas(&doc);
            assert_eq!(paginas.len(), 1, "{extensao}");
            let (largura, altura, pixels) = imagem_da_pagina(&doc, paginas[0]);
            assert_eq!((largura, altura), (23, 11), "{extensao}");
            assert_eq!(pixels.len(), 23 * 11 * 3, "{extensao}");
            // JPEG e WebP com perda: o primeiro pixel só precisa ficar perto.
            let esperado = origem.get_pixel(0, 0).0;
            let diferenca: u32 = (0..3)
                .map(|canal| u32::from(esperado[canal]).abs_diff(u32::from(pixels[canal])))
                .sum();
            assert!(diferenca < 40, "{extensao}: diferença {diferenca}");
        }
    }

    #[test]
    fn alfa_e_composto_sobre_branco() {
        let mut imagem = RgbaImage::from_pixel(2, 1, Rgba([255, 0, 0, 128]));
        imagem.put_pixel(1, 0, Rgba([0, 0, 255, 0]));
        let bytes = bytes_de(&DynamicImage::ImageRgba8(imagem), ImageFormat::Png);
        let pdf = pdf_de_imagem(&bytes, "png", &OpcoesDeImagem::default()).unwrap();
        let doc = documento(&pdf);
        let pagina = paginas(&doc)[0];
        let (_, _, pixels) = imagem_da_pagina(&doc, pagina);
        assert_eq!(&pixels[..3], &[255, 127, 127]);
        assert_eq!(&pixels[3..6], &[255, 255, 255]);
    }

    #[test]
    fn imagem_colorida_guarda_laranja_e_amarelo() {
        let mut imagem = RgbImage::from_pixel(2, 1, Rgb([255, 255, 0]));
        imagem.put_pixel(0, 0, Rgb([255, 128, 0]));
        let bytes = bytes_de(&DynamicImage::ImageRgb8(imagem), ImageFormat::Png);
        let pdf = pdf_de_imagem(&bytes, "png", &OpcoesDeImagem::default()).unwrap();
        let doc = documento(&pdf);
        let (_, _, pixels) = imagem_da_pagina(&doc, paginas(&doc)[0]);
        assert_eq!(pixels, vec![255, 128, 0, 255, 255, 0]);
    }

    #[test]
    fn tiff_de_duas_paginas_vira_duas_paginas() {
        let mut bytes = Cursor::new(Vec::new());
        {
            let mut codificador = tiff::encoder::TiffEncoder::new(&mut bytes).unwrap();
            for cor in [Rgb([255, 128, 0]), Rgb([0, 64, 255])] {
                let quadro = RgbImage::from_pixel(4, 3, cor);
                codificador
                    .write_image::<tiff::encoder::colortype::RGB8>(4, 3, quadro.as_raw())
                    .unwrap();
            }
        }
        let bytes = bytes.into_inner();
        let pdf = pdf_de_imagem(&bytes, "tif", &OpcoesDeImagem::default()).unwrap();
        let doc = documento(&pdf);
        let paginas = paginas(&doc);
        assert_eq!(paginas.len(), 2);
        for (indice, cor) in [[255, 128, 0], [0, 64, 255]].into_iter().enumerate() {
            let (_, _, pixels) = imagem_da_pagina(&doc, paginas[indice]);
            assert_eq!(&pixels[..3], &cor);
        }
    }

    #[test]
    fn pdf_derivado_abre_em_pdf_origem() {
        let bytes = bytes_de(
            &DynamicImage::ImageRgb8(rgb_de_teste(5, 4)),
            ImageFormat::Png,
        );
        let pdf = pdf_de_imagem(&bytes, "png", &OpcoesDeImagem::default()).unwrap();
        let origem = PdfOrigem::carregar(&pdf).unwrap();
        assert_eq!(origem.total_paginas(), 1);
    }

    #[test]
    fn imagem_em_pe_fica_em_pagina_retrato() {
        let bytes = bytes_de(
            &DynamicImage::ImageRgb8(rgb_de_teste(4, 9)),
            ImageFormat::Png,
        );
        let pdf = pdf_de_imagem(&bytes, "png", &OpcoesDeImagem::default()).unwrap();
        let doc = documento(&pdf);
        assert_eq!(caixa(&doc, paginas(&doc)[0]), (595.0, 842.0));
    }

    #[test]
    fn desenho_fica_centralizado_e_sem_distorcer() {
        let bytes = bytes_de(
            &DynamicImage::ImageRgb8(rgb_de_teste(400, 200)),
            ImageFormat::Png,
        );
        let pdf = pdf_de_imagem(&bytes, "png", &OpcoesDeImagem::default()).unwrap();
        let doc = documento(&pdf);
        let pagina = paginas(&doc)[0];
        let texto = conteudo(&doc, pagina);
        // 400x200 na A4 paisagem: 842x595 com margem de 168.85 em cima e embaixo.
        assert!(
            texto.contains("842.0000 0 0 421.0000 0.0000 87.0000"),
            "{texto}"
        );
    }

    #[test]
    fn dpi_do_arquivo_muda_o_tamanho_fisico() {
        let quadro = Quadro {
            largura: 800,
            altura: 600,
            rgb: vec![0; 800 * 600 * 3],
            dpi: Some((300.0, 300.0)),
        };
        let (largura, altura) = tamanho_do_desenho(&quadro, 842.0, 595.0, true);
        assert!((largura - 192.0).abs() < 0.001, "{largura}");
        assert!((altura - 144.0).abs() < 0.001, "{altura}");
        let (sem_dpi, _) = tamanho_do_desenho(&quadro, 842.0, 595.0, false);
        assert!((sem_dpi - 793.33).abs() < 0.01, "{sem_dpi}");
    }

    #[test]
    fn dpi_maior_que_a_folha_e_reduzido_sem_distorcer() {
        let quadro = Quadro {
            largura: 4000,
            altura: 2000,
            rgb: vec![0; 4000 * 2000 * 3],
            dpi: Some((10.0, 10.0)),
        };
        let (largura, altura) = tamanho_do_desenho(&quadro, 842.0, 595.0, true);
        assert!((largura / altura - 2.0).abs() < 0.001);
        assert!(largura <= 842.0 && altura <= 595.0);
    }

    #[test]
    fn limites_de_tamanho_e_de_pixels_sao_recusados() {
        assert_eq!(conferir_tamanho(0), Err(ErroImagem::Vazio));
        assert_eq!(
            conferir_tamanho(LIMITE_BYTES_IMAGEM + 1),
            Err(ErroImagem::Grande)
        );
        assert_eq!(conferir_tamanho(LIMITE_BYTES_IMAGEM), Ok(()));
        assert_eq!(conferir_dimensoes(0, 10), Err(ErroImagem::Vazio));
        assert_eq!(
            conferir_dimensoes(10_001, 10_000),
            Err(ErroImagem::QuadroGrande)
        );
        assert_eq!(
            conferir_dimensoes(100_000, 1_001),
            Err(ErroImagem::QuadroGrande)
        );
        assert_eq!(conferir_dimensoes(10_000, 10_000), Ok(()));
    }

    #[test]
    fn o_teto_de_alocacao_cabe_o_limite_de_pixels() {
        // RGB de 16 bits ocupa 6 bytes por pixel; o limite de pixels continua valendo antes da
        // decodificação, então o teto não aceita mais pixels do que o limite declarado.
        assert!(LIMITE_PIXELS * 6 <= TETO_DE_ALOCACAO, "{TETO_DE_ALOCACAO}");
        assert_eq!(
            conferir_dimensoes(10_001, 10_000),
            Err(ErroImagem::QuadroGrande)
        );
    }

    #[test]
    fn vazio_e_formato_desconhecido_sao_recusados() {
        assert_eq!(
            pdf_de_imagem(&[], "png", &OpcoesDeImagem::default()),
            Err(ErroImagem::Vazio)
        );
        assert_eq!(
            pdf_de_imagem(b"lixo", "txt", &OpcoesDeImagem::default()),
            Err(ErroImagem::FormatoNaoSuportado)
        );
        assert!(matches!(
            pdf_de_imagem(b"lixo", "png", &OpcoesDeImagem::default()),
            Err(ErroImagem::Decodificacao(_))
        ));
    }

    #[test]
    fn cabecalho_quebrado_vira_recusa_sem_panico() {
        for extensao in ["png", "jpg", "bmp", "gif", "webp", "tif"] {
            let resultado = pdf_de_imagem(&[0xFF; 64], extensao, &OpcoesDeImagem::default());
            assert!(resultado.is_err(), "{extensao}");
        }
    }

    #[test]
    fn dpi_do_png_le_pixels_por_metro() {
        let mut bytes = vec![137, 80, 78, 71, 13, 10, 26, 10];
        bytes.extend(9u32.to_be_bytes());
        bytes.extend(b"pHYs");
        bytes.extend(11_811u32.to_be_bytes()); // 300 dpi
        bytes.extend(11_811u32.to_be_bytes());
        let posicao_da_unidade = bytes.len();
        bytes.push(1);
        bytes.extend([0, 0, 0, 0]);
        let (x, y) = dpi_do_png(&bytes).unwrap();
        assert!((x - 300.0).abs() < 0.1, "{x}");
        assert!((y - 300.0).abs() < 0.1, "{y}");
        let mut sem_metro = bytes.clone();
        sem_metro[posicao_da_unidade] = 2;
        assert_eq!(dpi_do_png(&sem_metro), None);
        assert_eq!(dpi_do_png(&bytes[..20]), None);
    }

    #[test]
    fn dpi_do_jpeg_le_o_jfif() {
        let mut bytes = vec![0xFF, 0xD8];
        bytes.extend([0xFF, 0xE0, 0x00, 0x10]);
        bytes.extend(b"JFIF\0");
        bytes.extend([1, 1, 1]);
        bytes.extend(300u16.to_be_bytes());
        bytes.extend(300u16.to_be_bytes());
        bytes.extend([0, 0]);
        let (x, y) = dpi_do_jpeg(&bytes).unwrap();
        assert!((x - 300.0).abs() < 0.001);
        assert!((y - 300.0).abs() < 0.001);
        assert_eq!(dpi_do_jpeg(&[0; 8]), None);
    }

    #[test]
    fn dpi_do_bmp_le_pixels_por_metro() {
        let mut bytes = vec![b'B', b'M'];
        bytes.resize(14, 0);
        bytes.extend(40u32.to_le_bytes());
        bytes.resize(38, 0);
        bytes.extend(11_811i32.to_le_bytes());
        bytes.extend(11_811i32.to_le_bytes());
        let (x, y) = dpi_do_bmp(&bytes).unwrap();
        assert!((x - 300.0).abs() < 0.1);
        assert!((y - 300.0).abs() < 0.1);
        assert_eq!(dpi_do_bmp(&[0; 8]), None);
    }
}
