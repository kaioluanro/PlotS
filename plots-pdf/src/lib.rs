//! Extração de páginas para um PDF de saída de impressão.

use std::collections::{HashMap, HashSet, VecDeque};
use std::rc::Rc;

use lopdf::{dictionary, Dictionary, Document, Object, ObjectId};
use plots_core::{Pagina, LIMITE_PAGINAS};

pub mod calibracao;
pub use calibracao::{pdf_calibracao, PapelCalibracao, PAGINAS_CALIBRACAO};

#[derive(Debug)]
pub enum ErroPdf {
    Pdf(lopdf::Error),
    Escrita(std::io::Error),
    Criptografado,
    DocumentoInvalido(&'static str),
    PaginaInexistente(usize),
}

impl std::fmt::Display for ErroPdf {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Pdf(e) => write!(f, "erro ao processar PDF: {e}"),
            Self::Escrita(e) => write!(f, "erro ao gravar PDF: {e}"),
            Self::Criptografado => f.write_str("PDF criptografado não é suportado"),
            Self::DocumentoInvalido(s) => write!(f, "PDF inválido: {s}"),
            Self::PaginaInexistente(n) => write!(f, "página {n} não existe no PDF"),
        }
    }
}
impl std::error::Error for ErroPdf {}
impl From<lopdf::Error> for ErroPdf {
    fn from(value: lopdf::Error) -> Self {
        Self::Pdf(value)
    }
}

/// Documento aberto sem senha. PDFs criptografados não são aceitos, nem os que só têm senha de
/// dono: a biblioteca de leitura decifra esses sozinha, com a senha vazia, e apaga `/Encrypt`.
pub struct PdfOrigem {
    documento: Document,
    paginas: Vec<ObjectId>,
}

impl PdfOrigem {
    pub fn carregar(bytes: &[u8]) -> Result<Self, ErroPdf> {
        let documento = Document::load_mem_with_options(
            bytes,
            lopdf::LoadOptions::with_max_decompressed_size(LIMITE_FLUXO_BYTES),
        )
        .map_err(erro_de_carga)?;
        if documento.is_encrypted()
            || documento.was_encrypted()
            || documento.trailer.has(b"Encrypt")
        {
            return Err(ErroPdf::Criptografado);
        }
        let root = documento.trailer.get(b"Root")?.as_reference()?;
        let catalogo = documento.get_object(root)?.as_dict()?;
        if let Ok(formulario) = catalogo.get(b"AcroForm") {
            let formulario = documento.dereference(formulario)?.1.as_dict()?;
            if let Ok(precisa) = formulario.get(b"NeedAppearances") {
                if documento.dereference(precisa)?.1.as_bool()? {
                    return Err(ErroPdf::DocumentoInvalido(
                        "formulário requer regeneração de aparências",
                    ));
                }
            }
        }
        let paginas = listar_paginas(&documento)?;
        if paginas.is_empty() || paginas.len() > LIMITE_PAGINAS {
            return Err(ErroPdf::DocumentoInvalido("quantidade de páginas inválida"));
        }
        Ok(Self { documento, paginas })
    }

    /// Indica se alguma página tem um campo de assinatura digital já aplicada. As passadas
    /// mostram a aparência dela, mas não a contêm.
    pub fn tem_assinatura_aplicada(&self) -> Result<bool, ErroPdf> {
        for id in &self.paginas {
            let pagina = self.documento.get_object(*id)?.as_dict()?;
            let Ok(anotacoes) = pagina.get(b"Annots") else {
                continue;
            };
            let anotacoes = self.documento.dereference(anotacoes)?.1.as_array()?;
            if anotacoes.len() > LIMITE_PAGINAS {
                return Err(ErroPdf::DocumentoInvalido("anotações demais"));
            }
            for anotacao in anotacoes {
                let Ok(dict) = self
                    .documento
                    .dereference(anotacao)
                    .and_then(|(_, o)| o.as_dict())
                else {
                    continue;
                };
                let widget =
                    dict.get(b"Subtype").ok().and_then(|v| v.as_name().ok()) == Some(b"Widget");
                if widget && campo_assinado(&self.documento, dict)? {
                    return Ok(true);
                }
            }
        }
        Ok(false)
    }

    pub fn total_paginas(&self) -> usize {
        self.paginas.len()
    }

    /// Tamanho físico de cada página, em milímetros, considerada sua rotação. Mede a área que
    /// o leitor mostra (`MediaBox` cortada pela `CropBox`), não a folha de sangria inteira.
    pub fn dimensoes_paginas(&self) -> Result<Vec<plots_core::DimensoesPagina>, ErroPdf> {
        let mut dimensoes = Vec::with_capacity(self.paginas.len());
        for id in &self.paginas {
            let dict = pagina_com_heranca(&self.documento, *id)?;
            let [x0, y0, x1, y1] = caixa_visivel(&self.documento, &dict)?;
            let unidade = dict
                .get(b"UserUnit")
                .ok()
                .map(|v| {
                    self.documento
                        .dereference(v)
                        .and_then(|(_, o)| o.as_float())
                })
                .transpose()?
                .unwrap_or(1.0);
            if !unidade.is_finite() || !(0.0..=75_000.0).contains(&unidade) || unidade == 0.0 {
                return Err(ErroPdf::DocumentoInvalido("UserUnit inválido"));
            }
            let rotacao = dict
                .get(b"Rotate")
                .ok()
                .map(|v| self.documento.dereference(v).and_then(|(_, o)| o.as_i64()))
                .transpose()?
                .unwrap_or(0);
            if rotacao % 90 != 0 {
                return Err(ErroPdf::DocumentoInvalido("rotação inválida"));
            }
            let largura = (x1 - x0) * f64::from(unidade) * 25.4 / 72.0;
            let altura = (y1 - y0) * f64::from(unidade) * 25.4 / 72.0;
            let (largura, altura) = if rotacao.rem_euclid(180) == 90 {
                (altura, largura)
            } else {
                (largura, altura)
            };
            dimensoes.push(plots_core::DimensoesPagina::new(largura, altura));
        }
        Ok(dimensoes)
    }

    /// Monta uma passada. O branco usa a geometria da página indicada em
    /// `pagina_tamanho_branco` (número original começando em 1).
    pub fn gerar_passada(
        &self,
        paginas: &[Pagina],
        pagina_tamanho_branco: usize,
    ) -> Result<Vec<u8>, ErroPdf> {
        if paginas.is_empty() || paginas.len() > LIMITE_PAGINAS {
            return Err(ErroPdf::DocumentoInvalido("passada vazia ou excessiva"));
        }
        let proibidos = referencias_arvore_original(&self.documento, &self.paginas)?;
        let mut orcamento = Orcamento::novo();
        let mut analise = AnaliseForms::default();
        let mut saida = self.documento.clone();
        let pages_id = saida.new_object_id();
        let mut kids = Vec::with_capacity(paginas.len());
        let mut campos = HashMap::new();
        let mut campos_raiz = Vec::new();
        for pagina in paginas {
            let numero = pagina.numero.unwrap_or(pagina_tamanho_branco);
            let id = self
                .paginas
                .get(
                    numero
                        .checked_sub(1)
                        .ok_or(ErroPdf::PaginaInexistente(numero))?,
                )
                .ok_or(ErroPdf::PaginaInexistente(numero))?;
            let mut dict = pagina_com_heranca(&self.documento, *id)?;
            retirar_dados_fora_do_desenho(&mut dict);
            if pagina.numero.is_some() {
                // A análise não guarda bytes: cada página tem o orçamento inteiro.
                orcamento.reiniciar();
                podar_recursos_da_pagina(&self.documento, &mut dict, &mut orcamento, &mut analise)?;
            }
            let user_unit = dict.get(b"UserUnit").ok().cloned();
            if let Some(valor) = &user_unit {
                let numero = self.documento.dereference(valor)?.1.as_float()?;
                if !numero.is_finite() || !(0.0..=75_000.0).contains(&numero) || numero == 0.0 {
                    return Err(ErroPdf::DocumentoInvalido("UserUnit inválido"));
                }
            }
            let rotacao = dict
                .get(b"Rotate")
                .ok()
                .map(|r| self.documento.dereference(r).and_then(|(_, o)| o.as_i64()))
                .transpose()?
                .unwrap_or(0);
            if pagina.numero.is_none() {
                // A página branca contém um ponto muito discreto, evitando que
                // alguns drivers a descartem como completamente vazia.
                let media = dict.get(b"MediaBox")?.clone();
                let crop = dict.get(b"CropBox").ok().cloned();
                let [x0, y0, x1, y1] = caixa_visivel(&self.documento, &dict)?;
                let deslocamento = ((x1 - x0).min(y1 - y0) * 0.01).min(1.0);
                let tamanho = (deslocamento * 0.1).min(0.1);
                let comando = format!(
                    "q 0.98 g {:.4} {:.4} {:.4} {:.4} re f Q",
                    x0 + deslocamento,
                    y0 + deslocamento,
                    tamanho,
                    tamanho
                );
                let stream = lopdf::Stream::new(dictionary! {}, comando.into_bytes());
                let stream_id = saida.add_object(stream);
                dict = dictionary! { "Type" => "Page", "MediaBox" => media, "Resources" => dictionary! {}, "Contents" => stream_id };
                if let Some(crop) = crop {
                    dict.set("CropBox", crop);
                }
                if let Some(user_unit) = user_unit {
                    dict.set("UserUnit", user_unit);
                }
            }
            if rotacao % 90 != 0 {
                return Err(ErroPdf::DocumentoInvalido("rotação inválida"));
            }
            dict.set(
                "Rotate",
                (rotacao.rem_euclid(360) + if pagina.girar_180 { 180 } else { 0 }) % 360,
            );
            let pagina_id = saida.new_object_id();
            if pagina.numero.is_some() {
                clonar_anotacoes(
                    &self.documento,
                    &mut saida,
                    &mut dict,
                    pagina_id,
                    &mut campos,
                    &mut campos_raiz,
                )?;
            }
            dict.set("Parent", pages_id);
            saida.objects.insert(pagina_id, Object::Dictionary(dict));
            kids.push(Object::Reference(pagina_id));
        }
        let mut paginas_novas: HashSet<ObjectId> = kids
            .iter()
            .filter_map(|kid| kid.as_reference().ok())
            .collect();
        paginas_novas.insert(pages_id);
        saida.objects.insert(
            pages_id,
            Object::Dictionary(
                dictionary! { "Type" => "Pages", "Count" => kids.len() as i64, "Kids" => kids },
            ),
        );
        let mut catalogo_dict = dictionary! { "Type" => "Catalog", "Pages" => pages_id };
        if !campos_raiz.is_empty() {
            let mut acroform = dictionary! { "Fields" => campos_raiz.into_iter().map(Object::Reference).collect::<Vec<_>>() };
            let catalogo_origem = self.documento.trailer.get(b"Root")?.as_reference()?;
            let catalogo_origem = self.documento.get_object(catalogo_origem)?.as_dict()?;
            if let Ok(original) = catalogo_origem.get(b"AcroForm") {
                let original = self.documento.dereference(original)?.1.as_dict()?;
                for chave in [b"DA".as_slice(), b"DR", b"Q"] {
                    if let Ok(valor) = original.get(chave) {
                        acroform.set(chave, valor.clone());
                    }
                }
            }
            // As aparências existentes são obrigatórias; não geramos outras.
            acroform.set("NeedAppearances", false);
            catalogo_dict.set("AcroForm", saida.add_object(acroform));
        }
        let catalogo = saida.add_object(catalogo_dict);
        paginas_novas.insert(catalogo);
        saida.trailer.set("Root", catalogo);
        // Não conservar metadados nem objetos inalcançáveis do documento original.
        saida.trailer.remove(b"Info");
        saida.trailer.remove(b"ID");
        remover_inacessiveis(&mut saida, &proibidos, &paginas_novas)?;
        let mut bytes = Vec::new();
        saida.save_to(&mut bytes).map_err(ErroPdf::Escrita)?;
        Ok(bytes)
    }
}

/// Face de uma folha final: página de origem (começando em 1), ou `None` para uma face
/// em branco, e se a página aparece girada 180° dentro da folha.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct FaceFinal {
    pub pagina: Option<usize>,
    pub girada: bool,
}

/// Folha final: a frente fica à esquerda e o verso à direita. Face ausente sai em branco.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct FolhaFinal {
    pub frente: Option<FaceFinal>,
    pub verso: Option<FaceFinal>,
}

/// Maior dimensão aceita para uma página do PDF, em pontos (limite da especificação PDF).
const LIMITE_DIMENSAO_PT: f64 = 14_400.0;

/// Matriz afim no formato do PDF: (x, y) vira (a·x + c·y + e, b·x + d·y + f).
type Afim = [f64; 6];

/// Aplica `primeiro` e depois `depois` a um ponto.
fn compor(primeiro: Afim, depois: Afim) -> Afim {
    let [a1, b1, c1, d1, e1, f1] = primeiro;
    let [a2, b2, c2, d2, e2, f2] = depois;
    [
        a2 * a1 + c2 * b1,
        b2 * a1 + d2 * b1,
        a2 * c1 + c2 * d1,
        b2 * c1 + d2 * d1,
        a2 * e1 + c2 * f1 + e2,
        b2 * e1 + d2 * f1 + f2,
    ]
}

/// Geometria visível de uma página: caixa (MediaBox ∩ CropBox), unidade e rotação.
#[derive(Clone, Debug)]
struct Geometria {
    caixa: [f64; 4],
    unidade: f64,
    rotacao: i64,
}

impl Geometria {
    /// Tamanho exibido, em pontos, considerando a rotação da página.
    fn tamanho_exibido(&self) -> Result<(f64, f64), ErroPdf> {
        let largura = (self.caixa[2] - self.caixa[0]) * self.unidade;
        let altura = (self.caixa[3] - self.caixa[1]) * self.unidade;
        let (largura, altura) = if self.rotacao % 180 == 90 {
            (altura, largura)
        } else {
            (largura, altura)
        };
        let cabe = |valor: f64| valor.is_finite() && valor > 0.0 && valor <= LIMITE_DIMENSAO_PT;
        if !cabe(largura) || !cabe(altura) {
            return Err(ErroPdf::DocumentoInvalido(
                "página grande demais para folha final",
            ));
        }
        Ok((largura, altura))
    }
}

/// Face pronta para montar: geometria, origem, orientação e tamanho exibido.
struct FacePronta {
    geometria: Geometria,
    id: Option<ObjectId>,
    girada: bool,
    largura: f64,
    altura: f64,
}

impl FacePronta {
    /// Matriz que leva a caixa da página original até a posição (ox, oy) da folha.
    fn matriz(&self, ox: f64, oy: f64) -> Afim {
        let s = self.geometria.unidade;
        let [x0, y0, x1, y1] = self.geometria.caixa;
        let base: Afim = match self.geometria.rotacao {
            90 => [0.0, -s, s, 0.0, -s * y0, s * x1],
            180 => [-s, 0.0, 0.0, -s, s * x1, s * y1],
            270 => [0.0, s, -s, 0.0, s * y1, -s * x0],
            _ => [s, 0.0, 0.0, s, -s * x0, -s * y0],
        };
        let girar = if self.girada {
            [-1.0, 0.0, 0.0, -1.0, self.largura, self.altura]
        } else {
            [1.0, 0.0, 0.0, 1.0, 0.0, 0.0]
        };
        compor(compor(base, girar), [1.0, 0.0, 0.0, 1.0, ox, oy])
    }
}

fn reais(valores: &[f64]) -> Vec<Object> {
    valores.iter().map(|v| Object::Real(*v as f32)).collect()
}

/// Maior tamanho, depois de descomprimido, de um fluxo de conteúdo (ou de fluxo de objetos na
/// abertura). Páginas reais ficam em poucos MiB; 64 MiB deixa folga e ainda impede que um
/// fluxo pequeno vire gigabytes na memória.
const LIMITE_FLUXO_BYTES: usize = 64 << 20;

/// Soma máxima do conteúdo descomprimido lido para montar um PDF de saída (passada ou folhas
/// finais): 256 MiB cobrem centenas de páginas densas e barram um arquivo com muitos fluxos
/// grandes.
const LIMITE_TOTAL_BYTES: usize = 256 << 20;

/// Profundidade máxima de formulários dentro de formulários ao procurar recursos usados.
/// Documentos reais raramente passam de 4; 16 dá folga e limita a recursão.
const LIMITE_FORMULARIOS_ANINHADOS: usize = 16;

/// Nomes distintos aceitos numa varredura (página ou formulário), e na união deles. Passando
/// disso, a página ou o formulário vira "manter tudo", sem erro: nenhum documento real tem
/// dezenas de milhares de recursos nomeados no desenho de uma só página.
const LIMITE_NOMES_DISTINTOS: usize = 65_536;

/// Teto, por saída, do conteúdo descomprimido lido só para a análise. Depois dele, as páginas
/// restantes deixam de ser podadas (nunca recusadas).
const LIMITE_ANALISE_BYTES: usize = 1 << 30;

/// Teto, por saída, de nomes percorridos na análise. Depois dele, deixa-se de podar.
const LIMITE_VISITAS_NOMES: usize = 50_000_000;

const MSG_FLUXO_GRANDE: &str = "fluxo de conteúdo descomprimido passa do limite de 64 MiB";
const MSG_TOTAL_GRANDE: &str =
    "conteúdo descomprimido total passa do limite de 256 MiB para um PDF de saída";

/// Quanto conteúdo descomprimido ainda pode ser lido ao montar uma saída.
struct Orcamento {
    restante: usize,
}

impl Orcamento {
    fn novo() -> Self {
        Self {
            restante: LIMITE_TOTAL_BYTES,
        }
    }

    /// Devolve o total inteiro: usado quando o conteúdo lido não fica retido na saída.
    fn reiniciar(&mut self) {
        self.restante = LIMITE_TOTAL_BYTES;
    }

    /// Descomprime o fluxo respeitando o limite por fluxo e o que resta do total. O segundo
    /// valor diz se o resultado é suspeito: fluxo com filtro e dados que decodifica para nada,
    /// o que a biblioteca de leitura devolve sem erro quando os dados estão corrompidos. Fica
    /// um risco residual: um Flate truncado devolve só a parte lida, sem sinal de erro, e os
    /// nomes que viriam depois do corte não podem ser vistos.
    fn decodificar(&mut self, fluxo: &lopdf::Stream) -> Result<(Vec<u8>, bool), ErroPdf> {
        let limite = self.restante.min(LIMITE_FLUXO_BYTES);
        match fluxo.decompressed_content_with_limit(limite) {
            Ok(bytes) => {
                self.restante = self.restante.saturating_sub(bytes.len());
                let suspeito =
                    bytes.is_empty() && fluxo.dict.has(b"Filter") && !fluxo.content.is_empty();
                Ok((bytes, suspeito))
            }
            Err(lopdf::Error::Decompress(lopdf::DecompressError::MemoryLimitExceeded {
                ..
            })) => Err(ErroPdf::DocumentoInvalido(
                if self.restante < LIMITE_FLUXO_BYTES {
                    MSG_TOTAL_GRANDE
                } else {
                    MSG_FLUXO_GRANDE
                },
            )),
            Err(outro) => Err(outro.into()),
        }
    }
}

/// Traduz a recusa por tamanho descomprimido na abertura; os demais erros seguem como estão.
fn erro_de_carga(erro: lopdf::Error) -> ErroPdf {
    match erro {
        lopdf::Error::Decompress(lopdf::DecompressError::MemoryLimitExceeded { .. }) => {
            ErroPdf::DocumentoInvalido(MSG_FLUXO_GRANDE)
        }
        outro => outro.into(),
    }
}

/// Caixa que o leitor mostra: `MediaBox` cortada pela `CropBox`, quando há.
fn caixa_visivel(doc: &Document, pagina: &Dictionary) -> Result<[f64; 4], ErroPdf> {
    let [mut x0, mut y0, mut x1, mut y1] = caixa(doc, pagina.get(b"MediaBox")?)?.map(f64::from);
    if let Ok(crop) = pagina.get(b"CropBox") {
        let [cx0, cy0, cx1, cy1] = caixa(doc, crop)?.map(f64::from);
        x0 = x0.max(cx0);
        y0 = y0.max(cy0);
        x1 = x1.min(cx1);
        y1 = y1.min(cy1);
        if x0 >= x1 || y0 >= y1 {
            return Err(ErroPdf::DocumentoInvalido("CropBox fora de MediaBox"));
        }
    }
    Ok([x0, y0, x1, y1])
}

/// Troca os recursos da página pelos que o desenho dela usa. Se o conteúdo não puder ser
/// analisado, os recursos ficam como estavam; estourar um limite de tamanho é erro.
fn podar_recursos_da_pagina(
    doc: &Document,
    pagina: &mut Dictionary,
    orcamento: &mut Orcamento,
    analise: &mut AnaliseForms,
) -> Result<(), ErroPdf> {
    let Ok(recursos) = pagina.get(b"Resources").cloned() else {
        return Ok(());
    };
    if analise.bytes_restantes == 0 {
        return Ok(());
    }
    let (conteudo, legivel) = match conteudo_da_pagina(doc, pagina, orcamento) {
        Ok(lido) => lido,
        Err(ErroPdf::Pdf(_)) => return Ok(()),
        Err(outro) => return Err(outro),
    };
    let podados = recursos_podados(
        doc,
        &recursos,
        pagina,
        legivel.then_some(conteudo.as_slice()),
        orcamento,
        analise,
    )?;
    pagina.set("Resources", podados);
    Ok(())
}

/// Nomes (`/Nome`, com `#XX` decodificado) que aparecem em qualquer ponto dos bytes, sem
/// distinguir texto, comentário nem dado de imagem. Pode sobrar nome, nunca faltar: é a
/// varredura que decide o que ainda pode ser desenhado. Um `#` que não é seguido de dois
/// dígitos hexadecimais fica no nome, e o trecho anterior a ele também entra, porque a
/// biblioteca de leitura encerra o nome ali. Devolve `None` quando passa de `max_nomes`
/// nomes distintos ou esgota `visitas` (uma por `/` encontrada).
fn nomes_nos_bytes(
    bytes: &[u8],
    max_nomes: usize,
    visitas: &mut usize,
) -> Option<HashSet<Vec<u8>>> {
    let separador = |b: u8| {
        matches!(
            b,
            0 | 9
                | 10
                | 12
                | 13
                | 32
                | b'('
                | b')'
                | b'<'
                | b'>'
                | b'['
                | b']'
                | b'{'
                | b'}'
                | b'/'
                | b'%'
        )
    };
    let hexa = |b: u8| (b as char).to_digit(16);
    let mut nomes: HashSet<Vec<u8>> = HashSet::new();
    let mut i = 0;
    while i < bytes.len() {
        if bytes[i] != b'/' {
            i += 1;
            continue;
        }
        *visitas = visitas.checked_sub(1)?;
        i += 1;
        let mut nome = Vec::new();
        let mut prefixo = None;
        while i < bytes.len() && !separador(bytes[i]) {
            let decodificado = if bytes[i] == b'#' {
                let digitos = (
                    bytes.get(i + 1).and_then(|&b| hexa(b)),
                    bytes.get(i + 2).and_then(|&b| hexa(b)),
                );
                match digitos {
                    (Some(a), Some(b)) => Some((a * 16 + b) as u8),
                    _ => {
                        prefixo.get_or_insert_with(|| nome.clone());
                        None
                    }
                }
            } else {
                None
            };
            match decodificado {
                Some(byte) => {
                    nome.push(byte);
                    i += 3;
                }
                None => {
                    nome.push(bytes[i]);
                    i += 1;
                }
            }
        }
        for candidato in prefixo.into_iter().chain(std::iter::once(nome)) {
            if !nomes.contains(&candidato) {
                nomes.insert(candidato);
            }
        }
        if nomes.len() > max_nomes {
            return None;
        }
    }
    Some(nomes)
}

/// O que se sabe de um formulário: os nomes que aparecem no conteúdo dele e se ele (ou os
/// recursos dele) exige manter todos os recursos da página.
struct FormAnalisado {
    nomes: HashSet<Vec<u8>>,
    manter_tudo: bool,
}

/// Formulários já analisados, compartilhados entre as páginas de uma mesma saída: um
/// formulário repetido em cada página é decodificado e cobrado uma vez. Guarda também o que
/// ainda cabe nos tetos da análise; passando deles, deixa-se de podar.
struct AnaliseForms {
    formularios: HashMap<ObjectId, Rc<FormAnalisado>>,
    bytes_restantes: usize,
    visitas_restantes: usize,
    max_nomes: usize,
}

impl Default for AnaliseForms {
    fn default() -> Self {
        Self::com_limites(
            LIMITE_ANALISE_BYTES,
            LIMITE_VISITAS_NOMES,
            LIMITE_NOMES_DISTINTOS,
        )
    }
}

impl AnaliseForms {
    fn com_limites(bytes: usize, visitas: usize, max_nomes: usize) -> Self {
        Self {
            formularios: HashMap::new(),
            bytes_restantes: bytes,
            visitas_restantes: visitas,
            max_nomes,
        }
    }

    /// Varre `bytes`; `None` se a varredura estourou um teto (e então não se poda mais).
    fn varrer(&mut self, bytes: &[u8]) -> Option<HashSet<Vec<u8>>> {
        if bytes.len() > self.bytes_restantes {
            self.bytes_restantes = 0;
            return None;
        }
        self.bytes_restantes -= bytes.len();
        let nomes = nomes_nos_bytes(bytes, self.max_nomes, &mut self.visitas_restantes);
        if nomes.is_none() {
            self.visitas_restantes = 0;
        }
        nomes
    }

    fn obter(
        &mut self,
        doc: &Document,
        id: ObjectId,
        fluxo: &lopdf::Stream,
        orcamento: &mut Orcamento,
    ) -> Result<Rc<FormAnalisado>, ErroPdf> {
        if let Some(analisado) = self.formularios.get(&id) {
            return Ok(Rc::clone(analisado));
        }
        let mut manter_tudo = fluxo
            .dict
            .get(b"Resources")
            .ok()
            .is_some_and(|r| recursos_exigem_manter(doc, r))
            || self.bytes_restantes == 0
            || self.visitas_restantes == 0;
        let mut nomes = HashSet::new();
        if !manter_tudo {
            match orcamento.decodificar(fluxo) {
                Ok((bytes, false)) => match self.varrer(&bytes) {
                    Some(achados) => nomes = achados,
                    None => manter_tudo = true,
                },
                Ok((_, true)) | Err(ErroPdf::Pdf(_)) => manter_tudo = true,
                Err(outro) => return Err(outro),
            }
        }
        let analisado = Rc::new(FormAnalisado { nomes, manter_tudo });
        self.formularios.insert(id, Rc::clone(&analisado));
        Ok(analisado)
    }
}

/// Recursos que o desenho pode usar sem nomear no conteúdo: padrões, máscaras suaves e
/// fontes Type3 (os desenhos delas podem usar os recursos da página, mesmo com `/Resources`
/// próprio). Qualquer dúvida na leitura conta como "exige".
fn recursos_exigem_manter(doc: &Document, recursos: &Object) -> bool {
    let Ok((_, Object::Dictionary(dict))) = doc.dereference(recursos) else {
        return true;
    };
    if dict.has(b"Pattern") {
        return true;
    }
    if let Ok(estados) = dict.get(b"ExtGState") {
        let Ok((_, Object::Dictionary(estados))) = doc.dereference(estados) else {
            return true;
        };
        for (_, estado) in estados.iter() {
            let Ok((_, Object::Dictionary(estado))) = doc.dereference(estado) else {
                return true;
            };
            if let Ok(mascara) = estado.get(b"SMask") {
                let sem_mascara = doc
                    .dereference(mascara)
                    .ok()
                    .and_then(|(_, o)| o.as_name().ok())
                    == Some(b"None");
                if !sem_mascara {
                    return true;
                }
            }
        }
    }
    if let Ok(fontes) = dict.get(b"Font") {
        let Ok((_, Object::Dictionary(fontes))) = doc.dereference(fontes) else {
            return true;
        };
        for (_, fonte) in fontes.iter() {
            let Ok((_, Object::Dictionary(fonte))) = doc.dereference(fonte) else {
                return true;
            };
            if fonte.get(b"Subtype").and_then(Object::as_name).ok() == Some(b"Type3") {
                return true;
            }
        }
    }
    false
}

/// Indica se alguma anotação da página tem aparência (`/AP`): ela pode desenhar com recursos
/// da página.
fn pagina_tem_aparencia(doc: &Document, pagina: &Dictionary) -> bool {
    let Ok(anotacoes) = pagina.get(b"Annots") else {
        return false;
    };
    let Ok((_, Object::Array(anotacoes))) = doc.dereference(anotacoes) else {
        return true;
    };
    anotacoes
        .iter()
        .any(|anotacao| match doc.dereference(anotacao) {
            Ok((_, Object::Dictionary(dict))) => dict.has(b"AP"),
            _ => true,
        })
}

/// Cópia dos recursos com, em `/XObject`, só os nomes que aparecem no conteúdo da página ou
/// no de formulários que ela pode alcançar. Na dúvida (conteúdo ilegível, tetos da análise
/// estourados, recursos que o desenho usa sem nomear), devolve os recursos originais.
/// `conteudo` é `None` quando a leitura do conteúdo não é confiável.
fn recursos_podados(
    doc: &Document,
    recursos: &Object,
    pagina: &Dictionary,
    conteudo: Option<&[u8]>,
    orcamento: &mut Orcamento,
    analise: &mut AnaliseForms,
) -> Result<Object, ErroPdf> {
    let Some(conteudo) = conteudo else {
        return Ok(recursos.clone());
    };
    let Ok((_, Object::Dictionary(dict))) = doc.dereference(recursos) else {
        return Ok(recursos.clone());
    };
    let Some(xobjetos) = dict
        .get(b"XObject")
        .ok()
        .and_then(|o| doc.dereference(o).ok())
        .and_then(|(_, o)| o.as_dict().ok())
    else {
        return Ok(Object::Dictionary(dict.clone()));
    };
    if recursos_exigem_manter(doc, recursos) || pagina_tem_aparencia(doc, pagina) {
        return Ok(recursos.clone());
    }
    let Some(mut usados) = analise.varrer(conteudo) else {
        return Ok(recursos.clone());
    };
    let mut fila: VecDeque<(Vec<u8>, usize)> = usados.iter().map(|n| (n.clone(), 0)).collect();
    let mut vistos = HashSet::new();
    while let Some((nome, profundidade)) = fila.pop_front() {
        analise.visitas_restantes = match analise.visitas_restantes.checked_sub(1) {
            Some(restantes) => restantes,
            None => return Ok(recursos.clone()),
        };
        let Some(valor) = xobjetos.get(&nome).ok() else {
            continue;
        };
        let Ok((Some(id), Object::Stream(forma))) = doc.dereference(valor) else {
            continue;
        };
        let eh_forma = forma.dict.get(b"Subtype").and_then(Object::as_name).ok() == Some(b"Form");
        if !eh_forma || !vistos.insert(id) {
            continue;
        }
        if profundidade + 1 > LIMITE_FORMULARIOS_ANINHADOS {
            return Err(ErroPdf::DocumentoInvalido(
                "formulários aninhados demais no desenho da página",
            ));
        }
        let analisado = analise.obter(doc, id, forma, orcamento)?;
        if analisado.manter_tudo {
            return Ok(recursos.clone());
        }
        for interno in &analisado.nomes {
            if !usados.contains(interno) {
                usados.insert(interno.clone());
                fila.push_back((interno.clone(), profundidade + 1));
            }
        }
        if usados.len() > analise.max_nomes {
            return Ok(recursos.clone());
        }
    }
    let mut mantidos = Dictionary::new();
    for (nome, valor) in xobjetos.iter() {
        if usados.contains(nome) {
            mantidos.set(nome.clone(), valor.clone());
        }
    }
    let mut novo = dict.clone();
    novo.set("XObject", mantidos);
    Ok(Object::Dictionary(novo))
}

/// Conteúdo decodificado de uma página, concatenando todos os fluxos de /Contents, e se a
/// leitura é confiável (nenhum fluxo suspeito de corrompido).
fn conteudo_da_pagina(
    doc: &Document,
    pagina: &Dictionary,
    orcamento: &mut Orcamento,
) -> Result<(Vec<u8>, bool), ErroPdf> {
    let Ok(contents) = pagina.get(b"Contents") else {
        return Ok((Vec::new(), true));
    };
    let itens: Vec<Object> = match doc.dereference(contents)?.1 {
        Object::Array(itens) => itens.clone(),
        outro => vec![outro.clone()],
    };
    if itens.len() > LIMITE_PAGINAS {
        return Err(ErroPdf::DocumentoInvalido("fluxos de conteúdo demais"));
    }
    let mut bytes = Vec::new();
    let mut legivel = true;
    for item in &itens {
        let fluxo = doc.dereference(item)?.1.as_stream()?;
        let (parte, suspeito) = orcamento.decodificar(fluxo)?;
        legivel &= !suspeito;
        bytes.extend(parte);
        bytes.push(b'\n');
    }
    Ok((bytes, legivel))
}

impl PdfOrigem {
    fn geometria_pagina(&self, numero: usize) -> Result<(Geometria, ObjectId), ErroPdf> {
        let indice = numero
            .checked_sub(1)
            .ok_or(ErroPdf::PaginaInexistente(numero))?;
        let id = *self
            .paginas
            .get(indice)
            .ok_or(ErroPdf::PaginaInexistente(numero))?;
        let dict = pagina_com_heranca(&self.documento, id)?;
        let [x0, y0, x1, y1] = caixa_visivel(&self.documento, &dict)?;
        let unidade = dict
            .get(b"UserUnit")
            .ok()
            .map(|v| {
                self.documento
                    .dereference(v)
                    .and_then(|(_, o)| o.as_float())
            })
            .transpose()?
            .unwrap_or(1.0);
        let unidade = f64::from(unidade);
        if !unidade.is_finite() || !(0.0..=75_000.0).contains(&unidade) || unidade == 0.0 {
            return Err(ErroPdf::DocumentoInvalido("UserUnit inválido"));
        }
        let rotacao = dict
            .get(b"Rotate")
            .ok()
            .map(|v| self.documento.dereference(v).and_then(|(_, o)| o.as_i64()))
            .transpose()?
            .unwrap_or(0);
        if rotacao % 90 != 0 {
            return Err(ErroPdf::DocumentoInvalido("rotação inválida"));
        }
        Ok((
            Geometria {
                caixa: [x0, y0, x1, y1],
                unidade,
                rotacao: rotacao.rem_euclid(360),
            },
            id,
        ))
    }

    fn face_pronta(
        &self,
        face: Option<FaceFinal>,
        branco: &Geometria,
    ) -> Result<FacePronta, ErroPdf> {
        let (geometria, id, girada) = match face {
            Some(FaceFinal {
                pagina: Some(numero),
                girada,
            }) => {
                let (geometria, id) = self.geometria_pagina(numero)?;
                (geometria, Some(id), girada)
            }
            _ => (branco.clone(), None, false),
        };
        let (largura, altura) = geometria.tamanho_exibido()?;
        Ok(FacePronta {
            geometria,
            id,
            girada,
            largura,
            altura,
        })
    }

    /// Monta um PDF com uma página por folha, em tamanho real. Cada página original entra
    /// como formulário, com recursos, caixa visível e rotação; anotações não são copiadas.
    /// A face sem página fica em branco, e `pagina_tamanho_branco` dá o tamanho dela.
    pub fn gerar_folhas_finais(
        &self,
        folhas: &[FolhaFinal],
        pagina_tamanho_branco: usize,
    ) -> Result<Vec<u8>, ErroPdf> {
        if folhas.is_empty() || folhas.len() > LIMITE_PAGINAS {
            return Err(ErroPdf::DocumentoInvalido("quantidade de folhas inválida"));
        }
        let (branco, _) = self.geometria_pagina(pagina_tamanho_branco)?;
        let proibidos = referencias_arvore_original(&self.documento, &self.paginas)?;
        let mut orcamento = Orcamento::novo();
        let mut analise = AnaliseForms::default();
        let mut saida = self.documento.clone();
        let pages_id = saida.new_object_id();
        let mut kids = Vec::with_capacity(folhas.len());
        let mut paginas_novas: HashSet<ObjectId> = HashSet::new();
        for folha in folhas {
            let frente = self.face_pronta(folha.frente, &branco)?;
            let verso = self.face_pronta(folha.verso, &branco)?;
            let largura = frente.largura + verso.largura;
            let altura = frente.altura.max(verso.altura);
            if !largura.is_finite() || largura > LIMITE_DIMENSAO_PT || !altura.is_finite() {
                return Err(ErroPdf::DocumentoInvalido("folha final grande demais"));
            }
            let mut xobjetos = Dictionary::new();
            let mut desenho = String::new();
            for (nome, face, ox) in [("Frente", &frente, 0.0), ("Verso", &verso, frente.largura)] {
                let Some(id) = face.id else { continue };
                let dict = pagina_com_heranca(&self.documento, id)?;
                let (conteudo, legivel) =
                    conteudo_da_pagina(&self.documento, &dict, &mut orcamento)?;
                let recursos = match dict.get(b"Resources") {
                    Ok(recursos) => recursos_podados(
                        &self.documento,
                        recursos,
                        &dict,
                        legivel.then_some(conteudo.as_slice()),
                        &mut orcamento,
                        &mut analise,
                    )?,
                    Err(_) => Object::Dictionary(dictionary! {}),
                };
                let [x0, y0, x1, y1] = face.geometria.caixa;
                let matriz = face.matriz(ox, altura - face.altura);
                let forma = dictionary! {
                    "Type" => "XObject",
                    "Subtype" => "Form",
                    "BBox" => reais(&[x0, y0, x1, y1]),
                    "Matrix" => reais(&matriz),
                    "Resources" => recursos,
                };
                let forma_id = saida.add_object(lopdf::Stream::new(forma, conteudo));
                paginas_novas.insert(forma_id);
                xobjetos.set(nome, Object::Reference(forma_id));
                desenho.push_str(&format!("q /{nome} Do Q\n"));
            }
            let conteudo_id =
                saida.add_object(lopdf::Stream::new(dictionary! {}, desenho.into_bytes()));
            let pagina_id = saida.new_object_id();
            saida.objects.insert(
                pagina_id,
                Object::Dictionary(dictionary! {
                    "Type" => "Page",
                    "Parent" => pages_id,
                    "MediaBox" => reais(&[0.0, 0.0, largura, altura]),
                    "Resources" => dictionary! { "XObject" => Object::Dictionary(xobjetos) },
                    "Contents" => conteudo_id,
                }),
            );
            paginas_novas.insert(pagina_id);
            kids.push(Object::Reference(pagina_id));
        }
        paginas_novas.insert(pages_id);
        saida.objects.insert(
            pages_id,
            Object::Dictionary(
                dictionary! { "Type" => "Pages", "Count" => kids.len() as i64, "Kids" => kids },
            ),
        );
        let catalogo = saida.add_object(dictionary! { "Type" => "Catalog", "Pages" => pages_id });
        paginas_novas.insert(catalogo);
        saida.trailer.set("Root", catalogo);
        saida.trailer.remove(b"Info");
        saida.trailer.remove(b"ID");
        remover_inacessiveis(&mut saida, &proibidos, &paginas_novas)?;
        let mut bytes = Vec::new();
        saida.save_to(&mut bytes).map_err(ErroPdf::Escrita)?;
        Ok(bytes)
    }
}

/// Entradas da página que não participam do que é impresso e costumam apontar de volta
/// para outras páginas: fios de artigo, dados privados de aplicativo e a chave da
/// estrutura marcada (a árvore de estrutura não é copiada, então a chave ficaria solta).
fn retirar_dados_fora_do_desenho(pagina: &mut Dictionary) {
    for chave in [b"B".as_slice(), b"PieceInfo", b"StructParents"] {
        pagina.remove(chave);
    }
}

/// Indica se o campo (ou algum campo acima dele) guarda uma assinatura digital já aplicada:
/// há um valor `/V` e ele é um dicionário de assinatura ou o tipo do campo (`/FT`, herdado
/// na subida até o campo pai) é `Sig`.
fn campo_assinado(origem: &Document, widget: &Dictionary) -> Result<bool, ErroPdf> {
    let mut atual = widget.clone();
    let mut valor_assinatura = None;
    let mut tipo_sig = None;
    for _ in 0..32 {
        if valor_assinatura.is_none() {
            if let Ok(valor) = atual.get(b"V") {
                valor_assinatura = Some(match origem.dereference(valor)?.1 {
                    Object::Dictionary(d) => matches!(
                        d.get(b"Type").ok().and_then(|v| v.as_name().ok()),
                        Some(b"Sig") | Some(b"DocTimeStamp")
                    ),
                    _ => false,
                });
            }
        }
        if tipo_sig.is_none() {
            if let Ok(tipo) = atual.get(b"FT") {
                tipo_sig = Some(origem.dereference(tipo)?.1.as_name().ok() == Some(b"Sig"));
            }
        }
        match atual.get(b"Parent").and_then(Object::as_reference) {
            Ok(pai) => atual = origem.get_object(pai)?.as_dict()?.clone(),
            Err(_) => {
                return Ok(match valor_assinatura {
                    Some(dicionario_sig) => dicionario_sig || tipo_sig == Some(true),
                    None => false,
                })
            }
        }
    }
    Err(ErroPdf::DocumentoInvalido(
        "ciclo ou profundidade excessiva em campos",
    ))
}

/// Texto do PDF (UTF-16BE com marca de ordem) para o comentário do carimbo.
fn texto_pdf(texto: &str) -> Object {
    let mut bytes = vec![0xFE, 0xFF];
    for unidade in texto.encode_utf16() {
        bytes.extend(unidade.to_be_bytes());
    }
    Object::String(bytes, lopdf::StringFormat::Hexadecimal)
}

const COMENTARIO_CARIMBO: &str = "Cópia para impressão: a assinatura digital original não está neste arquivo. / Print copy: the original digital signature is not in this file.";

/// Troca o campo de assinatura por um carimbo com a mesma aparência e a mesma área. O valor
/// da assinatura nunca é impresso, pode apontar de volta para o catálogo do documento e
/// deixaria de valer de qualquer forma, pois o conteúdo do arquivo muda. Como carimbo, a
/// aparência continua igual na impressão e nenhum visualizador a trata como campo pendente.
fn assinatura_como_carimbo(widget: &mut Dictionary) {
    for chave in [
        b"FT".as_slice(),
        b"V",
        b"T",
        b"TU",
        b"TM",
        b"Ff",
        b"Lock",
        b"SV",
        b"DA",
        b"DV",
        b"MK",
        b"AS",
        b"Parent",
        b"Kids",
        b"AA",
        b"A",
    ] {
        widget.remove(chave);
    }
    widget.set("Subtype", "Stamp");
    widget.set("Contents", texto_pdf(COMENTARIO_CARIMBO));
}

fn clonar_anotacoes(
    origem: &Document,
    saida: &mut Document,
    pagina: &mut Dictionary,
    pagina_id: ObjectId,
    campos: &mut HashMap<ObjectId, ObjectId>,
    campos_raiz: &mut Vec<ObjectId>,
) -> Result<(), ErroPdf> {
    let anotacoes = match pagina.get(b"Annots") {
        Ok(valor) => origem.dereference(valor)?.1.as_array()?,
        Err(_) => return Ok(()),
    };
    if anotacoes.len() > LIMITE_PAGINAS {
        return Err(ErroPdf::DocumentoInvalido("anotações demais"));
    }
    let mut novas = Vec::with_capacity(anotacoes.len());
    let mut originais = Vec::with_capacity(anotacoes.len());
    let mut mapa = HashMap::new();
    for anotacao in anotacoes {
        let (original_id, valor) = origem.dereference(anotacao)?;
        let dict = valor.as_dict()?.clone();
        let novo_id = saida.new_object_id();
        if let Some(original_id) = original_id {
            if mapa.insert(original_id, novo_id).is_some() {
                return Err(ErroPdf::DocumentoInvalido("anotação duplicada"));
            }
        }
        originais.push((novo_id, dict));
        novas.push(Object::Reference(novo_id));
    }
    for (novo_id, mut dict) in originais {
        // As passadas são documentos temporários: destinos internos apontariam
        // para outra ordem ou para páginas ausentes, então são removidos.
        dict.remove(b"Dest");
        let mut remover_acao = false;
        if let Ok(acao) = dict.get(b"A") {
            let acao = origem.dereference(acao)?.1.as_dict()?;
            if acao.get(b"S").and_then(Object::as_name)? == b"GoTo" {
                remover_acao = true;
            } else if acao.has(b"Next") {
                return Err(ErroPdf::DocumentoInvalido(
                    "cadeia de ações de anotação não suportada",
                ));
            }
        }
        if remover_acao {
            dict.remove(b"A");
        }
        let mut widget =
            dict.get(b"Subtype").ok().and_then(|v| v.as_name().ok()) == Some(b"Widget");
        if widget && campo_assinado(origem, &dict)? {
            validar_aparencia_widget(origem, &dict)?;
            assinatura_como_carimbo(&mut dict);
            widget = false;
        }
        if widget {
            validar_aparencia_widget(origem, &dict)?;
            if let Ok(parent) = dict.get(b"Parent") {
                let campo =
                    clonar_campo(origem, saida, parent.as_reference()?, campos, campos_raiz)?;
                adicionar_filho(saida, campo, novo_id)?;
                dict.set("Parent", campo);
            } else {
                campos_raiz.push(novo_id);
            }
        }
        for chave in [b"Parent".as_slice(), b"Popup", b"IRT"] {
            if widget && chave == b"Parent" {
                continue;
            }
            if let Ok(relacao) = dict.get(chave) {
                let id = relacao.as_reference()?;
                let novo = mapa.get(&id).ok_or(ErroPdf::DocumentoInvalido(
                    "relacionamento de anotação não suportado",
                ))?;
                dict.set(chave, *novo);
            }
        }
        dict.set("P", pagina_id);
        saida.objects.insert(novo_id, Object::Dictionary(dict));
    }
    pagina.set("Annots", novas);
    Ok(())
}

fn validar_aparencia_widget(origem: &Document, widget: &Dictionary) -> Result<(), ErroPdf> {
    let ap = widget
        .get(b"AP")
        .map_err(|_| ErroPdf::DocumentoInvalido("widget sem aparência de impressão"))?;
    let ap = origem.dereference(ap)?.1.as_dict()?;
    let normal = origem
        .dereference(
            ap.get(b"N")
                .map_err(|_| ErroPdf::DocumentoInvalido("aparência normal ausente"))?,
        )?
        .1;
    let escolhido =
        match normal {
            Object::Stream(_) => normal,
            Object::Dictionary(estados) => {
                let estado = widget
                    .get(b"AS")
                    .map_err(|_| ErroPdf::DocumentoInvalido("estado de aparência ausente"))?
                    .as_name()?;
                origem
                    .dereference(estados.get(estado).map_err(|_| {
                        ErroPdf::DocumentoInvalido("estado de aparência inexistente")
                    })?)?
                    .1
            }
            _ => return Err(ErroPdf::DocumentoInvalido("aparência normal inválida")),
        };
    escolhido.as_stream()?;
    Ok(())
}

fn adicionar_filho(saida: &mut Document, pai: ObjectId, filho: ObjectId) -> Result<(), ErroPdf> {
    let campo = saida.get_object_mut(pai)?.as_dict_mut()?;
    let mut kids = campo
        .get(b"Kids")
        .ok()
        .map(Object::as_array)
        .transpose()?
        .cloned()
        .unwrap_or_default();
    if kids.len() >= LIMITE_PAGINAS {
        return Err(ErroPdf::DocumentoInvalido("filhos de campo demais"));
    }
    kids.push(Object::Reference(filho));
    campo.set("Kids", kids);
    Ok(())
}

fn clonar_campo(
    origem: &Document,
    saida: &mut Document,
    inicial: ObjectId,
    campos: &mut HashMap<ObjectId, ObjectId>,
    raizes: &mut Vec<ObjectId>,
) -> Result<ObjectId, ErroPdf> {
    let mut atual = inicial;
    let mut vistos = HashSet::new();
    let mut cadeia = Vec::new();
    let mut pai_clonado = loop {
        if let Some(&id) = campos.get(&atual) {
            break Some(id);
        }
        if cadeia.len() >= 128 || !vistos.insert(atual) {
            return Err(ErroPdf::DocumentoInvalido(
                "ciclo ou profundidade excessiva em campos",
            ));
        }
        let dict = origem.get_object(atual)?.as_dict()?.clone();
        if dict.get(b"Subtype").ok().and_then(|v| v.as_name().ok()) == Some(b"Widget") {
            return Err(ErroPdf::DocumentoInvalido("campo pai Widget não suportado"));
        }
        let pai = dict
            .get(b"Parent")
            .ok()
            .map(Object::as_reference)
            .transpose()?;
        cadeia.push((atual, dict));
        match pai {
            Some(id) => atual = id,
            None => break None,
        }
    };
    for (original, mut dict) in cadeia.into_iter().rev() {
        dict.remove(b"Parent");
        dict.remove(b"Kids");
        dict.remove(b"P");
        if let Some(pai) = pai_clonado {
            dict.set("Parent", pai);
        }
        let id = saida.add_object(dict);
        campos.insert(original, id);
        if let Some(pai) = pai_clonado {
            adicionar_filho(saida, pai, id)?;
        } else {
            raizes.push(id);
        }
        pai_clonado = Some(id);
    }
    pai_clonado.ok_or(ErroPdf::DocumentoInvalido("campo inválido"))
}

fn listar_paginas(doc: &Document) -> Result<Vec<ObjectId>, ErroPdf> {
    enum No {
        Entrar(ObjectId, Option<ObjectId>, usize),
        Sair(usize, usize),
    }
    let catalogo = doc.trailer.get(b"Root")?.as_reference()?;
    let root = doc
        .get_object(catalogo)?
        .as_dict()?
        .get(b"Pages")?
        .as_reference()?;
    let mut pilha = vec![No::Entrar(root, None, 0)];
    let mut visitados = HashSet::new();
    let mut paginas = Vec::new();
    while let Some(no) = pilha.pop() {
        match no {
            No::Sair(inicio, esperado) => {
                if paginas.len() - inicio != esperado {
                    return Err(ErroPdf::DocumentoInvalido(
                        "contagem de páginas inconsistente",
                    ));
                }
            }
            No::Entrar(id, pai, profundidade) => {
                if profundidade > 256 || visitados.len() >= 2_000_000 || !visitados.insert(id) {
                    return Err(ErroPdf::DocumentoInvalido("árvore de páginas inválida"));
                }
                let dict = doc.get_object(id)?.as_dict()?;
                if let Some(pai) = pai {
                    if dict.get(b"Parent")?.as_reference()? != pai {
                        return Err(ErroPdf::DocumentoInvalido("pai de página inconsistente"));
                    }
                } else if dict.has(b"Parent") {
                    return Err(ErroPdf::DocumentoInvalido("raiz de páginas possui pai"));
                }
                match dict.get(b"Type")?.as_name()? {
                    b"Page" => {
                        paginas.push(id);
                        if paginas.len() > LIMITE_PAGINAS {
                            return Err(ErroPdf::DocumentoInvalido("páginas demais"));
                        }
                    }
                    b"Pages" => {
                        let contagem = usize::try_from(dict.get(b"Count")?.as_i64()?)
                            .map_err(|_| ErroPdf::DocumentoInvalido("contagem inválida"))?;
                        if contagem > LIMITE_PAGINAS {
                            return Err(ErroPdf::DocumentoInvalido("páginas demais"));
                        }
                        let kids_obj = doc.dereference(dict.get(b"Kids")?)?.1;
                        let kids = kids_obj.as_array()?;
                        if kids.len() > LIMITE_PAGINAS {
                            return Err(ErroPdf::DocumentoInvalido("filhos demais"));
                        }
                        pilha.push(No::Sair(paginas.len(), contagem));
                        for kid in kids.iter().rev() {
                            pilha.push(No::Entrar(kid.as_reference()?, Some(id), profundidade + 1));
                        }
                    }
                    _ => return Err(ErroPdf::DocumentoInvalido("tipo de página inválido")),
                }
            }
        }
    }
    Ok(paginas)
}

fn referencias_arvore_original(
    doc: &Document,
    paginas: &[ObjectId],
) -> Result<HashSet<ObjectId>, ErroPdf> {
    let mut proibidos = HashSet::new();
    for (&id, objeto) in &doc.objects {
        if let Ok(dict) = objeto.as_dict() {
            if let Ok(tipo) = dict.get(b"Type").and_then(Object::as_name) {
                if matches!(tipo, b"Page" | b"Pages" | b"Catalog") {
                    proibidos.insert(id);
                }
            }
        }
    }
    for &pagina in paginas {
        let mut atual = pagina;
        let mut vistos = HashSet::new();
        for _ in 0..256 {
            if !vistos.insert(atual) {
                return Err(ErroPdf::DocumentoInvalido("ciclo na árvore de páginas"));
            }
            proibidos.insert(atual);
            let dict = doc.get_object(atual)?.as_dict()?;
            match dict.get(b"Parent") {
                Ok(parent) => atual = parent.as_reference()?,
                Err(_) => break,
            }
        }
    }
    Ok(proibidos)
}

fn remover_inacessiveis(
    doc: &mut Document,
    proibidos: &HashSet<ObjectId>,
    paginas_novas: &HashSet<ObjectId>,
) -> Result<(), ErroPdf> {
    let mut pilha: Vec<(&Object, bool)> = doc
        .trailer
        .iter()
        .map(|(_, valor)| (valor, false))
        .collect();
    let mut ids = HashSet::new();
    let mut visitados = 0usize;
    while let Some((objeto, pagina_permitida)) = pilha.pop() {
        visitados += 1;
        if visitados > 2_000_000 {
            return Err(ErroPdf::DocumentoInvalido("objetos demais"));
        }
        match objeto {
            Object::Reference(id) if proibidos.contains(id) => {
                return Err(ErroPdf::DocumentoInvalido(
                    "a página usa referências a outras páginas do documento que não podem ser retiradas sem alterar a impressão",
                ))
            }
            Object::Reference(id) if ids.insert(*id) => pilha.push((
                doc.objects
                    .get(id)
                    .ok_or(ErroPdf::DocumentoInvalido("referência inexistente"))?,
                paginas_novas.contains(id),
            )),
            Object::Array(itens) => pilha.extend(itens.iter().map(|item| (item, false))),
            Object::Dictionary(dict) => {
                if !pagina_permitida
                    && dict
                        .get(b"Type")
                        .ok()
                        .and_then(|v| v.as_name().ok())
                        .is_some_and(|tipo| matches!(tipo, b"Page" | b"Pages" | b"Catalog"))
                {
                    return Err(ErroPdf::DocumentoInvalido(
                        "a página embute dados de outras páginas do documento que não podem ser retirados sem alterar a impressão",
                    ));
                }
                pilha.extend(dict.iter().map(|(_, valor)| (valor, false)));
            }
            Object::Stream(stream) => {
                pilha.extend(stream.dict.iter().map(|(_, valor)| (valor, false)))
            }
            _ => {}
        }
    }
    doc.objects.retain(|id, _| ids.contains(id));
    Ok(())
}

fn caixa(doc: &Document, objeto: &Object) -> Result<[f32; 4], ErroPdf> {
    let (_, objeto) = doc.dereference(objeto)?;
    let valores = objeto.as_array()?;
    if valores.len() != 4 {
        return Err(ErroPdf::DocumentoInvalido("caixa inválida"));
    }
    let mut resultado = [0.0; 4];
    for (indice, valor) in valores.iter().enumerate() {
        resultado[indice] = doc.dereference(valor)?.1.as_float()?;
        if !resultado[indice].is_finite() {
            return Err(ErroPdf::DocumentoInvalido("caixa inválida"));
        }
    }
    if resultado[0] >= resultado[2] || resultado[1] >= resultado[3] {
        return Err(ErroPdf::DocumentoInvalido("caixa inválida"));
    }
    Ok(resultado)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn fixture() -> Vec<u8> {
        let mut doc = Document::with_version("1.5");
        let root = doc.new_object_id();
        let s1 = doc.add_object(lopdf::Stream::new(dictionary! {}, b"1 2 m".to_vec()));
        let s2 = doc.add_object(lopdf::Stream::new(dictionary! {}, b"3 4 m".to_vec()));
        let p1 = doc.add_object(
            dictionary! {"Type" => "Page", "Parent" => root, "Marker" => 1, "Contents" => s1},
        );
        let p2 = doc.add_object(
            dictionary! {"Type" => "Page", "Parent" => root, "Marker" => 2, "Rotate" => 180, "Contents" => s2},
        );
        doc.objects.insert(
            root,
            Object::Dictionary(dictionary! {
                "Type" => "Pages", "Kids" => vec![Object::Reference(p1), Object::Reference(p2)],
                "Count" => 2, "MediaBox" => vec![0.into(), 0.into(), 200.into(), 300.into()],
                "CropBox" => vec![0.into(), 0.into(), 190.into(), 290.into()],
                "Resources" => dictionary! {}, "Rotate" => 90
            }),
        );
        let catalog = doc.add_object(dictionary! {"Type" => "Catalog", "Pages" => root});
        doc.trailer.set("Root", catalog);
        let mut bytes = Vec::new();
        doc.save_to(&mut bytes).unwrap();
        bytes
    }

    #[test]
    fn reabre_ordem_heranca_rotacao_e_branco() {
        let origem = PdfOrigem::carregar(&fixture()).unwrap();
        let paginas = [
            Pagina {
                numero: Some(2),
                girar_180: true,
            },
            Pagina {
                numero: Some(1),
                girar_180: true,
            },
            Pagina {
                numero: None,
                girar_180: true,
            },
        ];
        let bytes = origem.gerar_passada(&paginas, 1).unwrap();
        let saida = Document::load_mem(&bytes).unwrap();
        let ids: Vec<_> = saida.get_pages().into_values().collect();
        assert_eq!(ids.len(), 3);
        let p2 = saida.get_object(ids[0]).unwrap().as_dict().unwrap();
        assert_eq!(p2.get(b"Marker").unwrap().as_i64().unwrap(), 2);
        assert_eq!(p2.get(b"Rotate").unwrap().as_i64().unwrap(), 0);
        assert_eq!(saida.get_page_content(ids[0]), b"3 4 m\n");
        let p1 = saida.get_object(ids[1]).unwrap().as_dict().unwrap();
        assert_eq!(p1.get(b"Marker").unwrap().as_i64().unwrap(), 1);
        assert_eq!(p1.get(b"Rotate").unwrap().as_i64().unwrap(), 270);
        assert_eq!(p1.get(b"MediaBox").unwrap().as_array().unwrap().len(), 4);
        assert!(p1.has(b"Resources") && p1.has(b"CropBox"));
        let branco = saida.get_object(ids[2]).unwrap().as_dict().unwrap();
        assert!(!branco.has(b"Marker"));
        assert_eq!(branco.get(b"Rotate").unwrap().as_i64().unwrap(), 270);
        assert_eq!(
            branco.get(b"MediaBox").unwrap(),
            p1.get(b"MediaBox").unwrap()
        );
        assert!(branco.has(b"Contents"));
    }

    #[test]
    fn rejeita_pdf_invalido_e_paginas_inexistentes() {
        assert!(PdfOrigem::carregar(b"invalid pdf").is_err());
        let origem = PdfOrigem::carregar(&fixture()).unwrap();
        assert_eq!(origem.total_paginas(), 2);
        for numero in [0, 3] {
            assert!(matches!(
                origem.gerar_passada(
                    &[Pagina {
                        numero: Some(numero),
                        girar_180: false
                    }],
                    1
                ),
                Err(ErroPdf::PaginaInexistente(_))
            ));
        }
        assert!(origem.gerar_passada(&[], 1).is_err());
    }

    #[test]
    fn rejeita_referencia_inexistente() {
        let mut origem = PdfOrigem::carregar(&fixture()).unwrap();
        let id = origem.paginas[0];
        origem
            .documento
            .get_object_mut(id)
            .unwrap()
            .as_dict_mut()
            .unwrap()
            .set("Contents", Object::Reference((999999, 0)));
        assert!(matches!(
            origem.gerar_passada(
                &[Pagina {
                    numero: Some(1),
                    girar_180: false
                }],
                1
            ),
            Err(ErroPdf::DocumentoInvalido("referência inexistente"))
        ));
    }

    #[test]
    fn rejeita_pdf_criptografado() {
        let mut doc = Document::load_mem(&fixture()).unwrap();
        doc.trailer.set(
            "ID",
            vec![
                Object::string_literal("test-file-id"),
                Object::string_literal("test-file-id"),
            ],
        );
        let version = lopdf::EncryptionVersion::V1 {
            document: &doc,
            owner_password: "owner",
            user_password: "secret",
            permissions: lopdf::Permissions::default(),
        };
        let estado = lopdf::EncryptionState::try_from(version).unwrap();
        doc.encrypt(&estado).unwrap();
        let mut bytes = Vec::new();
        doc.save_to(&mut bytes).unwrap();
        assert!(PdfOrigem::carregar(&bytes).is_err());
    }

    #[test]
    fn rejeita_pdf_com_senha_de_dono_e_senha_de_usuario_vazia() {
        let mut doc = Document::load_mem(&fixture()).unwrap();
        doc.trailer.set(
            "ID",
            vec![
                Object::string_literal("test-file-id"),
                Object::string_literal("test-file-id"),
            ],
        );
        let version = lopdf::EncryptionVersion::V1 {
            document: &doc,
            owner_password: "owner",
            user_password: "",
            permissions: lopdf::Permissions::empty(),
        };
        let estado = lopdf::EncryptionState::try_from(version).unwrap();
        doc.encrypt(&estado).unwrap();
        let mut bytes = Vec::new();
        doc.save_to(&mut bytes).unwrap();
        assert!(matches!(
            PdfOrigem::carregar(&bytes),
            Err(ErroPdf::Criptografado)
        ));
    }

    #[test]
    fn clona_anotacao_e_nao_inclui_pagina_nao_selecionada() {
        let mut origem = PdfOrigem::carregar(&fixture()).unwrap();
        let pagina_original = origem.paginas[0];
        let pagina_excluida = origem.paginas[1];
        let conteudo_excluido = origem
            .documento
            .get_object(pagina_excluida)
            .unwrap()
            .as_dict()
            .unwrap()
            .get(b"Contents")
            .unwrap()
            .as_reference()
            .unwrap();
        let aparencia = origem.documento.add_object(lopdf::Stream::new(dictionary! { "Type" => "XObject", "Subtype" => "Form", "BBox" => vec![0.into(), 0.into(), 20.into(), 20.into()] }, b"q 0 0 1 rg 0 0 20 20 re f Q".to_vec()));
        let anotacao = origem
            .documento
            .add_object(dictionary! { "Type" => "Annot", "Subtype" => "Square", "P" => pagina_original, "F" => 4, "Rect" => vec![0.into(), 0.into(), 20.into(), 20.into()], "AP" => dictionary! { "N" => aparencia } });
        origem
            .documento
            .get_object_mut(pagina_original)
            .unwrap()
            .as_dict_mut()
            .unwrap()
            .set("Annots", vec![Object::Reference(anotacao)]);
        let bytes = origem
            .gerar_passada(
                &[Pagina {
                    numero: Some(1),
                    girar_180: false,
                }],
                1,
            )
            .unwrap();
        let saida = Document::load_mem(&bytes).unwrap();
        let id_saida = *saida.get_pages().values().next().unwrap();
        let folha = saida.get_object(id_saida).unwrap().as_dict().unwrap();
        let anotacao_saida = folha.get(b"Annots").unwrap().as_array().unwrap()[0]
            .as_reference()
            .unwrap();
        let anotacao_saida = saida.get_object(anotacao_saida).unwrap().as_dict().unwrap();
        assert_eq!(
            anotacao_saida.get(b"P").unwrap().as_reference().unwrap(),
            id_saida
        );
        assert_eq!(anotacao_saida.get(b"F").unwrap().as_i64().unwrap(), 4);
        let ap = anotacao_saida
            .get(b"AP")
            .unwrap()
            .as_dict()
            .unwrap()
            .get(b"N")
            .unwrap()
            .as_reference()
            .unwrap();
        assert_eq!(
            saida.get_object(ap).unwrap().as_stream().unwrap().content,
            b"q 0 0 1 rg 0 0 20 20 re f Q"
        );
        assert!(!saida.objects.contains_key(&pagina_excluida));
        assert!(!saida.objects.contains_key(&conteudo_excluido));
    }

    #[test]
    fn remove_destinos_internos_de_links_sem_incluir_pagina_excluida() {
        let mut origem = PdfOrigem::carregar(&fixture()).unwrap();
        let p1 = origem.paginas[0];
        let p2 = origem.paginas[1];
        let link = origem.documento.add_object(dictionary! { "Type" => "Annot", "Subtype" => "Link", "P" => p1, "Dest" => vec![Object::Reference(p2), Object::Name(b"Fit".to_vec())] });
        let acao = origem.documento.add_object(dictionary! {"S" => "GoTo", "D" => vec![Object::Reference(p2), Object::Name(b"Fit".to_vec())] });
        let link2 = origem.documento.add_object(
            dictionary! {"Type" => "Annot", "Subtype" => "Link", "P" => p1, "A" => acao},
        );
        origem
            .documento
            .get_object_mut(p1)
            .unwrap()
            .as_dict_mut()
            .unwrap()
            .set(
                "Annots",
                vec![Object::Reference(link), Object::Reference(link2)],
            );
        let bytes = origem
            .gerar_passada(
                &[Pagina {
                    numero: Some(1),
                    girar_180: false,
                }],
                1,
            )
            .unwrap();
        let doc = Document::load_mem(&bytes).unwrap();
        let pagina = *doc.get_pages().values().next().unwrap();
        let anotacoes = doc
            .get_object(pagina)
            .unwrap()
            .as_dict()
            .unwrap()
            .get(b"Annots")
            .unwrap()
            .as_array()
            .unwrap();
        for anotacao in anotacoes {
            let anotacao = doc
                .get_object(anotacao.as_reference().unwrap())
                .unwrap()
                .as_dict()
                .unwrap();
            assert!(!anotacao.has(b"Dest") && !anotacao.has(b"A"));
        }
        assert!(!doc.objects.contains_key(&p2));
    }

    #[test]
    fn relaciona_popup_clonado_e_rejeita_pai_widget_externo() {
        let mut origem = PdfOrigem::carregar(&fixture()).unwrap();
        let p1 = origem.paginas[0];
        let nota = origem.documento.new_object_id();
        let popup = origem.documento.add_object(
            dictionary! {"Type" => "Annot", "Subtype" => "Popup", "P" => p1, "Parent" => nota},
        );
        origem.documento.objects.insert(
            nota,
            Object::Dictionary(
                dictionary! {"Type" => "Annot", "Subtype" => "Text", "P" => p1, "Popup" => popup},
            ),
        );
        origem
            .documento
            .get_object_mut(p1)
            .unwrap()
            .as_dict_mut()
            .unwrap()
            .set(
                "Annots",
                vec![Object::Reference(nota), Object::Reference(popup)],
            );
        let bytes = origem
            .gerar_passada(
                &[Pagina {
                    numero: Some(1),
                    girar_180: false,
                }],
                1,
            )
            .unwrap();
        let doc = Document::load_mem(&bytes).unwrap();
        let pagina = *doc.get_pages().values().next().unwrap();
        let lista = doc
            .get_object(pagina)
            .unwrap()
            .as_dict()
            .unwrap()
            .get(b"Annots")
            .unwrap()
            .as_array()
            .unwrap();
        let n = lista[0].as_reference().unwrap();
        let p = lista[1].as_reference().unwrap();
        assert_eq!(
            doc.get_object(n)
                .unwrap()
                .as_dict()
                .unwrap()
                .get(b"Popup")
                .unwrap()
                .as_reference()
                .unwrap(),
            p
        );
        assert_eq!(
            doc.get_object(p)
                .unwrap()
                .as_dict()
                .unwrap()
                .get(b"Parent")
                .unwrap()
                .as_reference()
                .unwrap(),
            n
        );
        let campo = origem.documento.add_object(dictionary! {"FT" => "Tx"});
        let widget = origem
            .documento
            .add_object(dictionary! {"Type" => "Annot", "Subtype" => "Widget", "Parent" => campo});
        origem
            .documento
            .get_object_mut(p1)
            .unwrap()
            .as_dict_mut()
            .unwrap()
            .set("Annots", vec![Object::Reference(widget)]);
        assert!(matches!(
            origem.gerar_passada(
                &[Pagina {
                    numero: Some(1),
                    girar_180: false
                }],
                1
            ),
            Err(ErroPdf::DocumentoInvalido(
                "widget sem aparência de impressão"
            ))
        ));
    }

    #[test]
    fn clona_pai_widget_e_preserva_valor_sem_incluir_irmao_excluido() {
        let mut origem = PdfOrigem::carregar(&fixture()).unwrap();
        let p1 = origem.paginas[0];
        let p2 = origem.paginas[1];
        let pai = origem.documento.new_object_id();
        let aparencia = origem.documento.add_object(lopdf::Stream::new(dictionary! {"Type" => "XObject", "Subtype" => "Form", "BBox" => vec![0.into(), 0.into(), 10.into(), 10.into()]}, b"q Q".to_vec()));
        let w1 = origem.documento.add_object(dictionary! {"Type" => "Annot", "Subtype" => "Widget", "Parent" => pai, "P" => p1, "F" => 4, "Rect" => vec![0.into(), 0.into(), 10.into(), 10.into()], "AP" => dictionary! {"N" => aparencia}});
        let w2 = origem.documento.add_object(dictionary! {"Type" => "Annot", "Subtype" => "Widget", "Parent" => pai, "P" => p2, "AP" => dictionary! {"N" => aparencia}});
        origem.documento.objects.insert(pai, Object::Dictionary(dictionary! {"FT" => "Tx", "T" => Object::string_literal("nome"), "V" => Object::string_literal("valor"), "Kids" => vec![Object::Reference(w1), Object::Reference(w2)]}));
        origem
            .documento
            .get_object_mut(p1)
            .unwrap()
            .as_dict_mut()
            .unwrap()
            .set("Annots", vec![Object::Reference(w1)]);
        origem
            .documento
            .get_object_mut(p2)
            .unwrap()
            .as_dict_mut()
            .unwrap()
            .set("Annots", vec![Object::Reference(w2)]);
        let bytes = origem
            .gerar_passada(
                &[Pagina {
                    numero: Some(1),
                    girar_180: false,
                }],
                1,
            )
            .unwrap();
        let doc = Document::load_mem(&bytes).unwrap();
        let catalog = doc
            .get_object(doc.trailer.get(b"Root").unwrap().as_reference().unwrap())
            .unwrap()
            .as_dict()
            .unwrap();
        let acroform = doc
            .get_object(catalog.get(b"AcroForm").unwrap().as_reference().unwrap())
            .unwrap()
            .as_dict()
            .unwrap();
        let campo = acroform.get(b"Fields").unwrap().as_array().unwrap()[0]
            .as_reference()
            .unwrap();
        let campo = doc.get_object(campo).unwrap().as_dict().unwrap();
        assert_eq!(campo.get(b"V").unwrap().as_str().unwrap(), b"valor");
        let kids = campo.get(b"Kids").unwrap().as_array().unwrap();
        assert_eq!(kids.len(), 1);
        let widget = doc
            .get_object(kids[0].as_reference().unwrap())
            .unwrap()
            .as_dict()
            .unwrap();
        assert!(widget.has(b"AP"));
        assert!(!doc.objects.contains_key(&p2));
        assert!(!doc.objects.contains_key(&w2));
    }

    #[test]
    fn rejeita_formulario_com_valor_e_aparencia_potencialmente_divergentes() {
        let mut doc = Document::load_mem(&fixture()).unwrap();
        let pagina = *doc.get_pages().values().next().unwrap();
        let aparencia_antiga = doc.add_object(lopdf::Stream::new(dictionary! {"Type" => "XObject", "Subtype" => "Form", "BBox" => vec![0.into(), 0.into(), 10.into(), 10.into()]}, b"% valor antigo".to_vec()));
        let widget = doc.add_object(dictionary! {"Type" => "Annot", "Subtype" => "Widget", "FT" => "Tx", "P" => pagina, "V" => Object::string_literal("valor novo"), "AP" => dictionary! {"N" => aparencia_antiga}});
        doc.get_object_mut(pagina)
            .unwrap()
            .as_dict_mut()
            .unwrap()
            .set("Annots", vec![Object::Reference(widget)]);
        let root = doc.trailer.get(b"Root").unwrap().as_reference().unwrap();
        doc.get_object_mut(root).unwrap().as_dict_mut().unwrap().set("AcroForm", dictionary! {"Fields" => vec![Object::Reference(widget)], "NeedAppearances" => true});
        let mut bytes = Vec::new();
        doc.save_to(&mut bytes).unwrap();
        assert!(matches!(
            PdfOrigem::carregar(&bytes),
            Err(ErroPdf::DocumentoInvalido(
                "formulário requer regeneração de aparências"
            ))
        ));
    }

    #[test]
    fn branco_usa_intersecao_das_caixas_e_rejeita_intersecao_vazia() {
        let mut origem = PdfOrigem::carregar(&fixture()).unwrap();
        let id = origem.paginas[0];
        origem
            .documento
            .get_object_mut(id)
            .unwrap()
            .as_dict_mut()
            .unwrap()
            .set(
                "CropBox",
                vec![(-10).into(), (-10).into(), 100.into(), 100.into()],
            );
        let bytes = origem
            .gerar_passada(
                &[Pagina {
                    numero: None,
                    girar_180: false,
                }],
                1,
            )
            .unwrap();
        let doc = Document::load_mem(&bytes).unwrap();
        let pagina = *doc.get_pages().values().next().unwrap();
        assert!(String::from_utf8(doc.get_page_content(pagina))
            .unwrap()
            .contains("1.0000 1.0000"));
        origem
            .documento
            .get_object_mut(id)
            .unwrap()
            .as_dict_mut()
            .unwrap()
            .set(
                "CropBox",
                vec![300.into(), 400.into(), 400.into(), 500.into()],
            );
        assert!(origem
            .gerar_passada(
                &[Pagina {
                    numero: None,
                    girar_180: false
                }],
                1
            )
            .is_err());
    }

    #[test]
    fn branco_preserva_user_unit_e_rejeita_valores_invalidos() {
        let mut origem = PdfOrigem::carregar(&fixture()).unwrap();
        let id = origem.paginas[0];
        origem
            .documento
            .get_object_mut(id)
            .unwrap()
            .as_dict_mut()
            .unwrap()
            .set("UserUnit", 2);
        let saida = origem
            .gerar_passada(
                &[Pagina {
                    numero: None,
                    girar_180: false,
                }],
                1,
            )
            .unwrap();
        let doc = Document::load_mem(&saida).unwrap();
        let pagina = *doc.get_pages().values().next().unwrap();
        assert_eq!(
            doc.get_object(pagina)
                .unwrap()
                .as_dict()
                .unwrap()
                .get(b"UserUnit")
                .unwrap()
                .as_i64()
                .unwrap(),
            2
        );
        origem
            .documento
            .get_object_mut(id)
            .unwrap()
            .as_dict_mut()
            .unwrap()
            .set("UserUnit", 0);
        assert!(origem
            .gerar_passada(
                &[Pagina {
                    numero: None,
                    girar_180: false
                }],
                1
            )
            .is_err());
    }

    #[test]
    fn rejeita_arvore_inconsistente_ciclica_e_profunda() {
        let base = PdfOrigem::carregar(&fixture()).unwrap();
        let root = base
            .documento
            .get_object(base.paginas[0])
            .unwrap()
            .as_dict()
            .unwrap()
            .get(b"Parent")
            .unwrap()
            .as_reference()
            .unwrap();
        let mut errado = base.documento.clone();
        errado
            .get_object_mut(root)
            .unwrap()
            .as_dict_mut()
            .unwrap()
            .set("Count", 3);
        assert!(listar_paginas(&errado).is_err());
        let mut ciclo = base.documento.clone();
        ciclo
            .get_object_mut(root)
            .unwrap()
            .as_dict_mut()
            .unwrap()
            .set("Kids", vec![Object::Reference(root)]);
        assert!(listar_paginas(&ciclo).is_err());
        let mut profundo = base.documento.clone();
        let mut proximo = base.paginas[0];
        for _ in 0..257 {
            let pai = profundo.new_object_id();
            profundo
                .get_object_mut(proximo)
                .unwrap()
                .as_dict_mut()
                .unwrap()
                .set("Parent", pai);
            profundo.objects.insert(pai, Object::Dictionary(dictionary! { "Type" => "Pages", "Kids" => vec![Object::Reference(proximo)], "Count" => 1 }));
            proximo = pai;
        }
        let catalog = profundo
            .trailer
            .get(b"Root")
            .unwrap()
            .as_reference()
            .unwrap();
        profundo
            .get_object_mut(catalog)
            .unwrap()
            .as_dict_mut()
            .unwrap()
            .set("Pages", proximo);
        assert!(listar_paginas(&profundo).is_err());
    }

    fn aproximados(a: &[f64], b: &[f64]) -> bool {
        a.len() == b.len() && a.iter().zip(b).all(|(x, y)| (x - y).abs() < 1e-3)
    }

    fn folha_de_teste(frente: Option<(usize, bool)>, verso: Option<(usize, bool)>) -> FolhaFinal {
        let face = |(pagina, girada): (usize, bool)| FaceFinal {
            pagina: Some(pagina),
            girada,
        };
        FolhaFinal {
            frente: frente.map(face),
            verso: verso.map(face),
        }
    }

    /// Caixa da folha e, para cada formulário de face, nome, matriz e conteúdo.
    fn ler_folha(doc: &Document, pagina: ObjectId) -> (Vec<f64>, Vec<(String, Vec<f64>, Vec<u8>)>) {
        let dict = doc.get_object(pagina).unwrap().as_dict().unwrap();
        let caixa = dict
            .get(b"MediaBox")
            .unwrap()
            .as_array()
            .unwrap()
            .iter()
            .map(|v| f64::from(v.as_float().unwrap()))
            .collect();
        let recursos = dict.get(b"Resources").unwrap().as_dict().unwrap();
        let xobjetos = recursos.get(b"XObject").unwrap().as_dict().unwrap();
        let faces = xobjetos
            .iter()
            .map(|(nome, valor)| {
                let forma = doc
                    .get_object(valor.as_reference().unwrap())
                    .unwrap()
                    .as_stream()
                    .unwrap();
                let matriz = forma
                    .dict
                    .get(b"Matrix")
                    .unwrap()
                    .as_array()
                    .unwrap()
                    .iter()
                    .map(|v| f64::from(v.as_float().unwrap()))
                    .collect();
                (
                    String::from_utf8_lossy(nome).into_owned(),
                    matriz,
                    forma.decompressed_content().unwrap(),
                )
            })
            .collect();
        (caixa, faces)
    }

    #[test]
    fn folhas_finais_somam_faces_alinham_no_topo_e_seguem_a_rotacao() {
        let origem = PdfOrigem::carregar(&fixture()).unwrap();
        let bytes = origem
            .gerar_folhas_finais(&[folha_de_teste(Some((1, false)), Some((2, false)))], 1)
            .unwrap();
        let saida = Document::load_mem(&bytes).unwrap();
        let paginas: Vec<_> = saida.get_pages().into_values().collect();
        assert_eq!(paginas.len(), 1);
        let (caixa, faces) = ler_folha(&saida, paginas[0]);
        assert!(aproximados(&caixa, &[0.0, 0.0, 480.0, 290.0]), "{caixa:?}");
        let frente = faces.iter().find(|f| f.0 == "Frente").unwrap();
        let verso = faces.iter().find(|f| f.0 == "Verso").unwrap();
        // Página 1 herda /Rotate 90 e fica 290 × 190, alinhada ao topo da folha.
        assert!(
            aproximados(&frente.1, &[0.0, -1.0, 1.0, 0.0, 0.0, 290.0]),
            "{:?}",
            frente.1
        );
        // Página 2 tem /Rotate 180, 190 × 290, à direita da frente.
        assert!(
            aproximados(&verso.1, &[-1.0, 0.0, 0.0, -1.0, 480.0, 290.0]),
            "{:?}",
            verso.1
        );
        assert_eq!(frente.2, b"1 2 m\n");
        assert_eq!(verso.2, b"3 4 m\n");
    }

    #[test]
    fn folhas_finais_com_face_girada_e_face_em_branco() {
        let origem = PdfOrigem::carregar(&fixture()).unwrap();
        // Frente girada 180° (página 1); verso em branco com a geometria da página 2.
        let bytes = origem
            .gerar_folhas_finais(&[folha_de_teste(Some((1, true)), None)], 2)
            .unwrap();
        let saida = Document::load_mem(&bytes).unwrap();
        let pagina = *saida.get_pages().values().next().unwrap();
        let (caixa, faces) = ler_folha(&saida, pagina);
        assert!(aproximados(&caixa, &[0.0, 0.0, 480.0, 290.0]), "{caixa:?}");
        assert_eq!(faces.len(), 1);
        assert_eq!(faces[0].0, "Frente");
        assert!(
            aproximados(&faces[0].1, &[0.0, 1.0, -1.0, 0.0, 290.0, 100.0]),
            "{:?}",
            faces[0].1
        );
    }

    #[test]
    fn folhas_finais_nao_levam_pagina_excluida_nem_anotacoes() {
        let mut origem = PdfOrigem::carregar(&fixture()).unwrap();
        let p1 = origem.paginas[0];
        let p2 = origem.paginas[1];
        let anotacao = origem.documento.add_object(dictionary! {"Type" => "Annot", "Subtype" => "Square", "P" => p1, "Rect" => vec![0.into(), 0.into(), 20.into(), 20.into()]});
        origem
            .documento
            .get_object_mut(p1)
            .unwrap()
            .as_dict_mut()
            .unwrap()
            .set("Annots", vec![Object::Reference(anotacao)]);
        let bytes = origem
            .gerar_folhas_finais(&[folha_de_teste(Some((1, false)), None)], 1)
            .unwrap();
        let saida = Document::load_mem(&bytes).unwrap();
        assert!(!saida.objects.contains_key(&p2));
        for objeto in saida.objects.values() {
            if let Ok(fluxo) = objeto.as_stream() {
                let conteudo = fluxo.decompressed_content().unwrap_or_default();
                assert!(!conteudo.windows(5).any(|w| w == b"3 4 m"));
            }
            if let Ok(dict) = objeto.as_dict() {
                assert!(
                    dict.get(b"Subtype").ok().and_then(|v| v.as_name().ok()) != Some(b"Square")
                );
            }
        }
    }

    #[test]
    fn folhas_finais_recusam_entradas_invalidas() {
        let origem = PdfOrigem::carregar(&fixture()).unwrap();
        assert!(origem.gerar_folhas_finais(&[], 1).is_err());
        assert!(matches!(
            origem.gerar_folhas_finais(&[folha_de_teste(Some((3, false)), None)], 1),
            Err(ErroPdf::PaginaInexistente(3))
        ));
        assert!(origem
            .gerar_folhas_finais(&[folha_de_teste(Some((1, false)), None)], 0)
            .is_err());
        let muitas = vec![folha_de_teste(Some((1, false)), None); LIMITE_PAGINAS + 1];
        assert!(origem.gerar_folhas_finais(&muitas, 1).is_err());
    }

    fn aparencia_vazia(doc: &mut Document) -> ObjectId {
        doc.add_object(lopdf::Stream::new(dictionary! {"Type" => "XObject", "Subtype" => "Form", "BBox" => vec![0.into(), 0.into(), 10.into(), 10.into()]}, b"q Q".to_vec()))
    }

    /// Assinatura no estilo de certificação: o dicionário de valor aponta de volta
    /// para o catálogo (e, por ele, para toda a árvore de páginas).
    fn assinatura_certificada(doc: &mut Document) -> ObjectId {
        let catalogo = doc.trailer.get(b"Root").unwrap().as_reference().unwrap();
        let referencia =
            dictionary! {"Type" => "SigRef", "TransformMethod" => "DocMDP", "Data" => catalogo};
        doc.add_object(dictionary! {"Type" => "Sig", "Filter" => "Adobe.PPKLite", "Reference" => vec![Object::Dictionary(referencia)]})
    }

    fn definir_anotacoes(doc: &mut Document, pagina: ObjectId, anotacoes: Vec<ObjectId>) {
        doc.get_object_mut(pagina)
            .unwrap()
            .as_dict_mut()
            .unwrap()
            .set(
                "Annots",
                anotacoes
                    .into_iter()
                    .map(Object::Reference)
                    .collect::<Vec<_>>(),
            );
    }

    fn passada_da_primeira(origem: &PdfOrigem) -> Result<Document, ErroPdf> {
        let bytes = origem.gerar_passada(
            &[Pagina {
                numero: Some(1),
                girar_180: false,
            }],
            1,
        )?;
        Ok(Document::load_mem(&bytes).unwrap())
    }

    fn sem_conteudo_da_pagina_excluida(doc: &Document, excluida: ObjectId) {
        assert!(!doc.objects.contains_key(&excluida));
        for objeto in doc.objects.values() {
            if let Ok(fluxo) = objeto.as_stream() {
                let conteudo = fluxo.decompressed_content().unwrap_or_default();
                assert!(!conteudo.windows(5).any(|w| w == b"3 4 m"));
            }
        }
    }

    #[test]
    fn assinatura_que_aponta_ao_catalogo_vira_carimbo_sem_o_valor() {
        let mut origem = PdfOrigem::carregar(&fixture()).unwrap();
        let (p1, p2) = (origem.paginas[0], origem.paginas[1]);
        let assinatura = assinatura_certificada(&mut origem.documento);
        let aparencia = aparencia_vazia(&mut origem.documento);
        let widget = origem.documento.add_object(dictionary! {"Type" => "Annot", "Subtype" => "Widget", "FT" => "Sig", "T" => Object::string_literal("assina"), "P" => p1, "F" => 4, "Rect" => vec![0.into(), 0.into(), 10.into(), 10.into()], "AP" => dictionary! {"N" => aparencia}, "V" => assinatura});
        definir_anotacoes(&mut origem.documento, p1, vec![widget]);
        let doc = passada_da_primeira(&origem).unwrap();
        let pagina = *doc.get_pages().values().next().unwrap();
        let anotacoes = doc
            .get_object(pagina)
            .unwrap()
            .as_dict()
            .unwrap()
            .get(b"Annots")
            .unwrap()
            .as_array()
            .unwrap();
        let saida = doc
            .get_object(anotacoes[0].as_reference().unwrap())
            .unwrap()
            .as_dict()
            .unwrap();
        assert!(saida.has(b"AP") && saida.has(b"Rect"));
        assert_eq!(saida.get(b"Subtype").unwrap().as_name().unwrap(), b"Stamp");
        assert!(!saida.has(b"V") && !saida.has(b"FT"));
        assert!(!doc.objects.contains_key(&assinatura));
        sem_conteudo_da_pagina_excluida(&doc, p2);
    }

    #[test]
    fn assinatura_no_campo_pai_vira_carimbo_sem_o_valor() {
        let mut origem = PdfOrigem::carregar(&fixture()).unwrap();
        let (p1, p2) = (origem.paginas[0], origem.paginas[1]);
        let assinatura = assinatura_certificada(&mut origem.documento);
        let aparencia = aparencia_vazia(&mut origem.documento);
        let pai = origem.documento.new_object_id();
        let widget = origem.documento.add_object(dictionary! {"Type" => "Annot", "Subtype" => "Widget", "Parent" => pai, "P" => p1, "AP" => dictionary! {"N" => aparencia}});
        origem.documento.objects.insert(pai, Object::Dictionary(dictionary! {"FT" => "Sig", "T" => Object::string_literal("campo"), "V" => assinatura, "Kids" => vec![Object::Reference(widget)]}));
        definir_anotacoes(&mut origem.documento, p1, vec![widget]);
        let doc = passada_da_primeira(&origem).unwrap();
        assert!(!doc.objects.contains_key(&assinatura));
        let mut carimbos = 0;
        for objeto in doc.objects.values() {
            if let Ok(dict) = objeto.as_dict() {
                assert!(!dict.has(b"V") && !dict.has(b"FT"));
                let carimbo = dict.get(b"Subtype").ok().and_then(|v| v.as_name().ok())
                    == Some(b"Stamp".as_slice());
                carimbos += usize::from(carimbo && dict.has(b"AP"));
            }
        }
        assert_eq!(carimbos, 1);
        sem_conteudo_da_pagina_excluida(&doc, p2);
    }

    #[test]
    fn valor_de_campo_comum_continua_sendo_preservado() {
        let mut origem = PdfOrigem::carregar(&fixture()).unwrap();
        let p1 = origem.paginas[0];
        let aparencia = aparencia_vazia(&mut origem.documento);
        let widget = origem.documento.add_object(dictionary! {"Type" => "Annot", "Subtype" => "Widget", "FT" => "Tx", "T" => Object::string_literal("a"), "V" => Object::string_literal("texto"), "P" => p1, "AP" => dictionary! {"N" => aparencia}});
        definir_anotacoes(&mut origem.documento, p1, vec![widget]);
        let doc = passada_da_primeira(&origem).unwrap();
        let tem_valor = doc.objects.values().any(|o| {
            o.as_dict()
                .ok()
                .and_then(|d| d.get(b"V").ok())
                .and_then(|v| v.as_str().ok())
                == Some(b"texto".as_slice())
        });
        assert!(tem_valor);
    }

    #[test]
    fn dados_da_pagina_que_voltam_as_paginas_sao_retirados() {
        let mut origem = PdfOrigem::carregar(&fixture()).unwrap();
        let (p1, p2) = (origem.paginas[0], origem.paginas[1]);
        let estrutura = origem
            .documento
            .add_object(dictionary! {"Type" => "StructElem", "S" => "P", "Pg" => p2});
        let fio = origem.documento.new_object_id();
        let conta = origem.documento.add_object(dictionary! {"Type" => "Bead", "T" => fio, "P" => p2, "R" => vec![0.into(), 0.into(), 5.into(), 5.into()]});
        origem.documento.objects.insert(
            fio,
            Object::Dictionary(dictionary! {"Type" => "Thread", "F" => conta}),
        );
        let pagina = origem
            .documento
            .get_object_mut(p1)
            .unwrap()
            .as_dict_mut()
            .unwrap();
        pagina.set("B", vec![Object::Reference(conta)]);
        pagina.set(
            "PieceInfo",
            dictionary! {"App" => dictionary! {"Data" => estrutura}},
        );
        pagina.set("StructParents", 0);
        let doc = passada_da_primeira(&origem).unwrap();
        let saida = doc
            .get_object(*doc.get_pages().values().next().unwrap())
            .unwrap()
            .as_dict()
            .unwrap();
        for chave in [b"B".as_slice(), b"PieceInfo", b"StructParents"] {
            assert!(!saida.has(chave));
        }
        assert!(saida.has(b"Contents"));
        assert!(!doc.objects.contains_key(&estrutura));
        sem_conteudo_da_pagina_excluida(&doc, p2);
    }

    #[test]
    fn referencia_no_desenho_da_pagina_continua_recusada_com_motivo_claro() {
        // O formulário só vai para a saída porque o conteúdo da página o desenha com `Do`.
        let mut origem = PdfOrigem::carregar(&fixture()).unwrap();
        let (p1, p2) = (origem.paginas[0], origem.paginas[1]);
        let forma = origem.documento.add_object(lopdf::Stream::new(dictionary! {"Type" => "XObject", "Subtype" => "Form", "BBox" => vec![0.into(), 0.into(), 10.into(), 10.into()], "Outra" => p2}, b"q Q".to_vec()));
        let desenho = origem
            .documento
            .add_object(lopdf::Stream::new(dictionary! {}, b"/F Do".to_vec()));
        let pagina = origem
            .documento
            .get_object_mut(p1)
            .unwrap()
            .as_dict_mut()
            .unwrap();
        pagina.set(
            "Resources",
            dictionary! {"XObject" => dictionary! {"F" => forma}},
        );
        pagina.set("Contents", desenho);
        let erro = passada_da_primeira(&origem).err().unwrap();
        assert!(matches!(erro, ErroPdf::DocumentoInvalido(_)));
        let texto = erro.to_string();
        assert!(!texto.contains("insegura"), "{texto}");
        assert!(texto.contains("outras páginas"), "{texto}");
    }

    fn texto_do_comentario(objeto: &Object) -> String {
        let bytes = objeto.as_str().unwrap();
        assert_eq!(&bytes[..2], &[0xFE, 0xFF]);
        let unidades: Vec<u16> = bytes[2..]
            .chunks(2)
            .map(|p| u16::from_be_bytes([p[0], p[1]]))
            .collect();
        String::from_utf16(&unidades).unwrap()
    }

    fn carimbos(doc: &Document) -> Vec<&Dictionary> {
        doc.objects
            .values()
            .filter_map(|o| o.as_dict().ok())
            .filter(|d| d.get(b"Subtype").ok().and_then(|v| v.as_name().ok()) == Some(b"Stamp"))
            .collect()
    }

    fn definir_acroform(doc: &mut Document, campos: Vec<ObjectId>) {
        let raiz = doc.trailer.get(b"Root").unwrap().as_reference().unwrap();
        let campos: Vec<Object> = campos.into_iter().map(Object::Reference).collect();
        doc.get_object_mut(raiz)
            .unwrap()
            .as_dict_mut()
            .unwrap()
            .set(
                "AcroForm",
                dictionary! {"Fields" => campos, "SigFlags" => 3, "DA" => Object::string_literal("/Helv 0 Tf")},
            );
    }

    #[test]
    fn acroform_novo_nao_declara_assinaturas_e_campo_comum_fica() {
        let mut origem = PdfOrigem::carregar(&fixture()).unwrap();
        let p1 = origem.paginas[0];
        let assinatura = assinatura_certificada(&mut origem.documento);
        let aparencia = aparencia_vazia(&mut origem.documento);
        let sig = origem.documento.add_object(dictionary! {"Type" => "Annot", "Subtype" => "Widget", "FT" => "Sig", "T" => Object::string_literal("s"), "P" => p1, "Rect" => vec![0.into(), 0.into(), 10.into(), 10.into()], "AP" => dictionary! {"N" => aparencia}, "V" => assinatura});
        let texto = origem.documento.add_object(dictionary! {"Type" => "Annot", "Subtype" => "Widget", "FT" => "Tx", "T" => Object::string_literal("t"), "V" => Object::string_literal("ok"), "P" => p1, "Rect" => vec![0.into(), 0.into(), 10.into(), 10.into()], "AP" => dictionary! {"N" => aparencia}});
        definir_anotacoes(&mut origem.documento, p1, vec![sig, texto]);
        definir_acroform(&mut origem.documento, vec![sig, texto]);
        let doc = passada_da_primeira(&origem).unwrap();
        let raiz = doc.trailer.get(b"Root").unwrap().as_reference().unwrap();
        let catalogo = doc.get_object(raiz).unwrap().as_dict().unwrap();
        let acroform = doc
            .get_object(catalogo.get(b"AcroForm").unwrap().as_reference().unwrap())
            .unwrap()
            .as_dict()
            .unwrap();
        assert!(!acroform.has(b"SigFlags"));
        assert!(acroform.has(b"DA"));
        let campos = acroform.get(b"Fields").unwrap().as_array().unwrap();
        assert_eq!(campos.len(), 1);
        let campo = doc
            .get_object(campos[0].as_reference().unwrap())
            .unwrap()
            .as_dict()
            .unwrap();
        assert_eq!(campo.get(b"V").unwrap().as_str().unwrap(), b"ok");
        assert_eq!(carimbos(&doc).len(), 1);
    }

    #[test]
    fn carimbo_da_assinatura_leva_o_comentario_mesmo_com_contents_anterior() {
        let mut origem = PdfOrigem::carregar(&fixture()).unwrap();
        let p1 = origem.paginas[0];
        let assinatura = assinatura_certificada(&mut origem.documento);
        let aparencia = aparencia_vazia(&mut origem.documento);
        let com = origem.documento.add_object(dictionary! {"Type" => "Annot", "Subtype" => "Widget", "FT" => "Sig", "P" => p1, "Contents" => Object::string_literal("antigo"), "Rect" => vec![0.into(), 0.into(), 10.into(), 10.into()], "AP" => dictionary! {"N" => aparencia}, "V" => assinatura});
        let sem = origem.documento.add_object(dictionary! {"Type" => "Annot", "Subtype" => "Widget", "FT" => "Sig", "P" => p1, "Rect" => vec![0.into(), 0.into(), 10.into(), 10.into()], "AP" => dictionary! {"N" => aparencia}, "V" => assinatura});
        definir_anotacoes(&mut origem.documento, p1, vec![com, sem]);
        let doc = passada_da_primeira(&origem).unwrap();
        let todos = carimbos(&doc);
        assert_eq!(todos.len(), 2);
        for carimbo in todos {
            assert_eq!(
                texto_do_comentario(carimbo.get(b"Contents").unwrap()),
                "Cópia para impressão: a assinatura digital original não está neste arquivo. / Print copy: the original digital signature is not in this file."
            );
        }
    }

    #[test]
    fn assinatura_sem_type_e_reconhecida_pelo_tipo_do_campo() {
        let mut origem = PdfOrigem::carregar(&fixture()).unwrap();
        let (p1, p2) = (origem.paginas[0], origem.paginas[1]);
        let catalogo = origem.documento.trailer.get(b"Root").unwrap().clone();
        let valor = origem.documento.add_object(dictionary! {"Filter" => "Adobe.PPKLite", "Name" => Object::string_literal("signatário"), "Reference" => vec![Object::Dictionary(dictionary! {"Data" => catalogo})]});
        let aparencia = aparencia_vazia(&mut origem.documento);
        let direto = origem.documento.add_object(dictionary! {"Type" => "Annot", "Subtype" => "Widget", "FT" => "Sig", "P" => p1, "Rect" => vec![0.into(), 0.into(), 10.into(), 10.into()], "AP" => dictionary! {"N" => aparencia}, "V" => valor});
        let pai = origem.documento.new_object_id();
        let filho = origem.documento.add_object(dictionary! {"Type" => "Annot", "Subtype" => "Widget", "Parent" => pai, "P" => p1, "Rect" => vec![0.into(), 0.into(), 10.into(), 10.into()], "AP" => dictionary! {"N" => aparencia}});
        origem.documento.objects.insert(pai, Object::Dictionary(dictionary! {"FT" => "Sig", "T" => Object::string_literal("pai"), "V" => valor, "Kids" => vec![Object::Reference(filho)]}));
        definir_anotacoes(&mut origem.documento, p1, vec![direto, filho]);
        assert!(origem.tem_assinatura_aplicada().unwrap());
        let doc = passada_da_primeira(&origem).unwrap();
        assert_eq!(carimbos(&doc).len(), 2);
        assert!(!doc.objects.contains_key(&valor));
        for objeto in doc.objects.values() {
            if let Ok(dict) = objeto.as_dict() {
                assert!(!dict.has(b"V") && !dict.has(b"FT"));
            }
        }
        sem_conteudo_da_pagina_excluida(&doc, p2);
    }

    #[test]
    fn detecta_assinatura_aplicada_somente_com_valor() {
        let mut origem = PdfOrigem::carregar(&fixture()).unwrap();
        assert!(!origem.tem_assinatura_aplicada().unwrap());
        let p2 = origem.paginas[1];
        let aparencia = aparencia_vazia(&mut origem.documento);
        let vazio = origem.documento.add_object(dictionary! {"Type" => "Annot", "Subtype" => "Widget", "FT" => "Sig", "P" => p2, "AP" => dictionary! {"N" => aparencia}});
        definir_anotacoes(&mut origem.documento, p2, vec![vazio]);
        assert!(!origem.tem_assinatura_aplicada().unwrap());
        let assinatura = assinatura_certificada(&mut origem.documento);
        origem
            .documento
            .get_object_mut(vazio)
            .unwrap()
            .as_dict_mut()
            .unwrap()
            .set("V", assinatura);
        assert!(origem.tem_assinatura_aplicada().unwrap());
    }

    #[test]
    fn id_do_trailer_nao_vai_para_as_copias() {
        let mut origem = PdfOrigem::carregar(&fixture()).unwrap();
        origem.documento.trailer.set(
            "ID",
            vec![
                Object::string_literal("identificador-a"),
                Object::string_literal("identificador-b"),
            ],
        );
        let passada = passada_da_primeira(&origem).unwrap();
        assert!(!passada.trailer.has(b"ID"));
        let bytes = origem
            .gerar_folhas_finais(&[folha_de_teste(Some((1, false)), None)], 1)
            .unwrap();
        let folhas = Document::load_mem(&bytes).unwrap();
        assert!(!folhas.trailer.has(b"ID"));
    }

    fn imagem(marca: &[u8]) -> lopdf::Stream {
        lopdf::Stream::new(
            dictionary! {"Type" => "XObject", "Subtype" => "Image", "Width" => 1, "Height" => 1,
            "ColorSpace" => "DeviceGray", "BitsPerComponent" => 8},
            marca.to_vec(),
        )
    }

    fn formulario(conteudo: &[u8], recursos: Option<Dictionary>) -> lopdf::Stream {
        let mut dict = dictionary! {"Type" => "XObject", "Subtype" => "Form",
        "BBox" => vec![0.into(), 0.into(), 10.into(), 10.into()]};
        if let Some(recursos) = recursos {
            dict.set("Resources", recursos);
        }
        lopdf::Stream::new(dict, conteudo.to_vec())
    }

    /// Documento de duas páginas com `/Resources` compartilhado, herdado do nó de páginas:
    /// a página 1 desenha o que `conteudo1` diz e a página 2 usa só `ImB`.
    fn documento_compartilhado(
        conteudo1: &[u8],
        extras: Vec<(&str, lopdf::Stream)>,
    ) -> (Document, ObjectId) {
        let mut doc = Document::with_version("1.5");
        let root = doc.new_object_id();
        let ima = doc.add_object(imagem(b"MARCA-IMAGEM-A"));
        let imb = doc.add_object(imagem(b"MARCA-IMAGEM-B"));
        let mut xobjetos = dictionary! {"ImA" => ima, "ImB" => imb};
        for (nome, fluxo) in extras {
            let id = doc.add_object(fluxo);
            xobjetos.set(nome, id);
        }
        let recursos = doc.add_object(dictionary! {"XObject" => xobjetos});
        let c1 = doc.add_object(lopdf::Stream::new(dictionary! {}, conteudo1.to_vec()));
        let c2 = doc.add_object(lopdf::Stream::new(dictionary! {}, b"q /ImB Do Q".to_vec()));
        let p1 = doc.add_object(dictionary! {"Type" => "Page", "Parent" => root, "Contents" => c1});
        let p2 = doc.add_object(dictionary! {"Type" => "Page", "Parent" => root, "Contents" => c2});
        doc.objects.insert(
            root,
            Object::Dictionary(dictionary! {
                "Type" => "Pages", "Kids" => vec![Object::Reference(p1), Object::Reference(p2)],
                "Count" => 2, "MediaBox" => vec![0.into(), 0.into(), 200.into(), 300.into()],
                "Resources" => recursos
            }),
        );
        let catalogo = doc.add_object(dictionary! {"Type" => "Catalog", "Pages" => root});
        doc.trailer.set("Root", catalogo);
        (doc, p1)
    }

    fn salvar(doc: &mut Document) -> Vec<u8> {
        let mut bytes = Vec::new();
        doc.save_to(&mut bytes).unwrap();
        bytes
    }

    fn contem_marca(doc: &Document, marca: &[u8]) -> bool {
        doc.objects.values().any(|objeto| {
            objeto.as_stream().is_ok_and(|fluxo| {
                let dados = fluxo.decompressed_content().unwrap_or_default();
                dados.windows(marca.len()).any(|w| w == marca)
            })
        })
    }

    fn primeira_pagina() -> [Pagina; 1] {
        [Pagina {
            numero: Some(1),
            girar_180: false,
        }]
    }

    /// Passada e folha final montadas só com a página 1, já reabertas.
    fn saidas_da_primeira(bytes: &[u8]) -> Vec<Document> {
        let origem = PdfOrigem::carregar(bytes).unwrap();
        let passada = origem.gerar_passada(&primeira_pagina(), 1).unwrap();
        let folhas = origem
            .gerar_folhas_finais(&[folha_de_teste(Some((1, false)), None)], 1)
            .unwrap();
        vec![
            Document::load_mem(&passada).unwrap(),
            Document::load_mem(&folhas).unwrap(),
        ]
    }

    #[test]
    fn imagem_so_da_pagina_excluida_nao_vai_para_a_saida() {
        let (mut doc, _) = documento_compartilhado(b"q /ImA Do Q", vec![]);
        let bytes = salvar(&mut doc);
        for saida in saidas_da_primeira(&bytes) {
            assert!(contem_marca(&saida, b"MARCA-IMAGEM-A"));
            assert!(!contem_marca(&saida, b"MARCA-IMAGEM-B"));
        }
    }

    #[test]
    fn formulario_aninhado_sem_recursos_mantem_o_que_ele_desenha() {
        let extras = vec![
            ("Fm", formulario(b"/ImC Do", None)),
            ("ImC", imagem(b"MARCA-IMAGEM-C")),
        ];
        let (mut doc, _) = documento_compartilhado(b"q /Fm Do Q", extras);
        let bytes = salvar(&mut doc);
        for saida in saidas_da_primeira(&bytes) {
            assert!(contem_marca(&saida, b"MARCA-IMAGEM-C"));
            assert!(!contem_marca(&saida, b"MARCA-IMAGEM-A"));
            assert!(!contem_marca(&saida, b"MARCA-IMAGEM-B"));
        }
    }

    /// Os nomes de um formulário contam mesmo quando ele traz `/Resources` sem `/XObject`
    /// (o leitor herda o da página) ou com o dicionário em objeto indireto.
    #[test]
    fn formulario_com_recursos_sem_xobject_mantem_o_que_ele_desenha() {
        let proprios = dictionary! {"ProcSet" => vec![Object::Name(b"PDF".to_vec())]};
        let extras = vec![("Fm", formulario(b"/ImA Do", Some(proprios)))];
        let (mut doc, _) = documento_compartilhado(b"q /Fm Do Q", extras);
        let bytes = salvar(&mut doc);
        for saida in saidas_da_primeira(&bytes) {
            assert!(contem_marca(&saida, b"MARCA-IMAGEM-A"));
            assert!(!contem_marca(&saida, b"MARCA-IMAGEM-B"));
        }
    }

    #[test]
    fn formulario_com_recursos_indiretos_mantem_o_que_ele_desenha() {
        let extras = vec![("Fm", formulario(b"/ImA Do", Some(dictionary! {})))];
        let (mut doc, _) = documento_compartilhado(b"q /Fm Do Q", extras);
        let indireto =
            doc.add_object(dictionary! {"ProcSet" => vec![Object::Name(b"PDF".to_vec())]});
        for objeto in doc.objects.values_mut() {
            if let Object::Stream(fluxo) = objeto {
                if fluxo.dict.has(b"Resources") {
                    fluxo.dict.set("Resources", indireto);
                }
            }
        }
        let bytes = salvar(&mut doc);
        for saida in saidas_da_primeira(&bytes) {
            assert!(contem_marca(&saida, b"MARCA-IMAGEM-A"));
        }
    }

    #[test]
    fn nome_depois_de_operando_negativo_e_mantido() {
        let (mut doc, _) = documento_compartilhado(b"--5 0 Td q /ImA Do Q", vec![]);
        let bytes = salvar(&mut doc);
        for saida in saidas_da_primeira(&bytes) {
            assert!(contem_marca(&saida, b"MARCA-IMAGEM-A"));
            assert!(!contem_marca(&saida, b"MARCA-IMAGEM-B"));
        }
    }

    #[test]
    fn nome_depois_de_parentese_solto_e_mantido() {
        let extras = vec![("ImC", imagem(b"MARCA-IMAGEM-C"))];
        let (mut doc, _) = documento_compartilhado(b"q /ImA Do Q ) q /ImC Do Q", extras);
        let bytes = salvar(&mut doc);
        for saida in saidas_da_primeira(&bytes) {
            assert!(contem_marca(&saida, b"MARCA-IMAGEM-A"));
            assert!(contem_marca(&saida, b"MARCA-IMAGEM-C"));
        }
    }

    #[test]
    fn nome_com_codigo_hexadecimal_e_reconhecido() {
        let extras = vec![("Im C", imagem(b"MARCA-IMAGEM-C"))];
        let (mut doc, _) = documento_compartilhado(b"q /Im#20C Do Q", extras);
        let bytes = salvar(&mut doc);
        for saida in saidas_da_primeira(&bytes) {
            assert!(contem_marca(&saida, b"MARCA-IMAGEM-C"));
            assert!(!contem_marca(&saida, b"MARCA-IMAGEM-B"));
        }
    }

    fn recursos_da_pagina(doc: &mut Document, pagina: ObjectId) -> &mut Dictionary {
        let pai = doc
            .get_object(pagina)
            .unwrap()
            .as_dict()
            .unwrap()
            .get(b"Parent")
            .unwrap()
            .as_reference()
            .unwrap();
        let recursos = doc
            .get_object(pai)
            .unwrap()
            .as_dict()
            .unwrap()
            .get(b"Resources")
            .unwrap()
            .as_reference()
            .unwrap();
        doc.get_object_mut(recursos).unwrap().as_dict_mut().unwrap()
    }

    fn mantem_a_imagem_a_sem_do(doc: &mut Document) {
        let bytes = salvar(doc);
        for saida in saidas_da_primeira(&bytes) {
            assert!(contem_marca(&saida, b"MARCA-IMAGEM-A"));
        }
    }

    #[test]
    fn mascara_suave_faz_manter_os_recursos() {
        let (mut doc, p1) = documento_compartilhado(b"q Q", vec![]);
        let grupo = doc.add_object(formulario(b"/ImA Do", None));
        recursos_da_pagina(&mut doc, p1).set(
            "ExtGState",
            dictionary! {"GS" => dictionary! {"SMask" => dictionary! {"Type" => "Mask", "S" => "Alpha", "G" => grupo}}},
        );
        mantem_a_imagem_a_sem_do(&mut doc);
    }

    #[test]
    fn padrao_faz_manter_os_recursos() {
        let (mut doc, p1) = documento_compartilhado(b"q Q", vec![]);
        let padrao = doc.add_object(lopdf::Stream::new(
            dictionary! {"PatternType" => 1, "PaintType" => 1, "TilingType" => 1,
            "BBox" => vec![0.into(), 0.into(), 4.into(), 4.into()],
            "XStep" => 4, "YStep" => 4, "Resources" => dictionary! {}},
            b"/ImA Do".to_vec(),
        ));
        recursos_da_pagina(&mut doc, p1).set("Pattern", dictionary! {"P1" => padrao});
        mantem_a_imagem_a_sem_do(&mut doc);
    }

    #[test]
    fn anotacao_com_aparencia_faz_manter_os_recursos() {
        let (mut doc, p1) = documento_compartilhado(b"q Q", vec![]);
        let aparencia = doc.add_object(formulario(b"/ImA Do", None));
        let anotacao = doc.add_object(dictionary! {"Type" => "Annot", "Subtype" => "Square",
        "Rect" => vec![0.into(), 0.into(), 10.into(), 10.into()],
        "AP" => dictionary! {"N" => aparencia}});
        definir_anotacoes(&mut doc, p1, vec![anotacao]);
        mantem_a_imagem_a_sem_do(&mut doc);
    }

    #[test]
    fn fonte_type3_sem_recursos_faz_manter_os_recursos() {
        let (mut doc, p1) = documento_compartilhado(b"q Q", vec![]);
        let fonte = doc.add_object(dictionary! {"Type" => "Font", "Subtype" => "Type3"});
        recursos_da_pagina(&mut doc, p1).set("Font", dictionary! {"F1" => fonte});
        mantem_a_imagem_a_sem_do(&mut doc);
    }

    #[test]
    fn trezentas_paginas_com_o_mesmo_formulario_geram_sem_erro() {
        let mut doc = Document::with_version("1.5");
        let root = doc.new_object_id();
        let forma = doc.add_object(formulario(&vec![b' '; 1 << 20], None));
        let recursos = doc.add_object(dictionary! {"XObject" => dictionary! {"F" => forma}});
        let conteudo = doc.add_object(lopdf::Stream::new(dictionary! {}, b"/F Do".to_vec()));
        let kids: Vec<ObjectId> = (0..300)
            .map(|_| {
                doc.add_object(
                    dictionary! {"Type" => "Page", "Parent" => root, "Contents" => conteudo},
                )
            })
            .collect();
        doc.objects.insert(
            root,
            Object::Dictionary(dictionary! {
                "Type" => "Pages", "Count" => 300,
                "Kids" => kids.iter().map(|k| Object::Reference(*k)).collect::<Vec<_>>(),
                "MediaBox" => vec![0.into(), 0.into(), 200.into(), 300.into()],
                "Resources" => recursos
            }),
        );
        let catalogo = doc.add_object(dictionary! {"Type" => "Catalog", "Pages" => root});
        doc.trailer.set("Root", catalogo);
        let bytes = salvar(&mut doc);
        let origem = PdfOrigem::carregar(&bytes).unwrap();
        let todas: Vec<Pagina> = (1..=300)
            .map(|n| Pagina {
                numero: Some(n),
                girar_180: false,
            })
            .collect();
        origem.gerar_passada(&todas, 1).unwrap();
        let folhas: Vec<FolhaFinal> = (1..=300)
            .map(|n| folha_de_teste(Some((n, false)), None))
            .collect();
        origem.gerar_folhas_finais(&folhas, 1).unwrap();
    }

    fn gerar_passada_primeira(bytes: &[u8]) -> Document {
        let origem = PdfOrigem::carregar(bytes).unwrap();
        let passada = origem.gerar_passada(&primeira_pagina(), 1).unwrap();
        Document::load_mem(&passada).unwrap()
    }

    fn lixo_flate() -> lopdf::Stream {
        lopdf::Stream::new(
            dictionary! {"Filter" => "FlateDecode"},
            b"isto nao e um fluxo zlib valido".to_vec(),
        )
    }

    #[test]
    fn conteudo_da_pagina_com_flate_corrompido_mantem_todos_os_recursos() {
        let (mut doc, p1) = documento_compartilhado(b"q /ImA Do Q", vec![]);
        let c = doc.add_object(lixo_flate());
        doc.get_object_mut(p1)
            .unwrap()
            .as_dict_mut()
            .unwrap()
            .set("Contents", c);
        let bytes = salvar(&mut doc);
        let saida = gerar_passada_primeira(&bytes);
        assert!(contem_marca(&saida, b"MARCA-IMAGEM-A"));
        assert!(contem_marca(&saida, b"MARCA-IMAGEM-B"));
        let origem = PdfOrigem::carregar(&bytes).unwrap();
        let folhas = origem
            .gerar_folhas_finais(&[folha_de_teste(Some((1, false)), None)], 1)
            .unwrap();
        let folhas = Document::load_mem(&folhas).unwrap();
        assert!(contem_marca(&folhas, b"MARCA-IMAGEM-A"));
        assert!(contem_marca(&folhas, b"MARCA-IMAGEM-B"));
    }

    #[test]
    fn formulario_com_flate_corrompido_mantem_todos_os_recursos() {
        let (mut doc, p1) = documento_compartilhado(b"q /Fm Do Q", vec![]);
        let forma = doc.add_object({
            let mut f = lixo_flate();
            f.dict.set("Type", "XObject");
            f.dict.set("Subtype", "Form");
            f.dict
                .set("BBox", vec![0.into(), 0.into(), 10.into(), 10.into()]);
            f
        });
        recursos_da_pagina(&mut doc, p1)
            .get_mut(b"XObject")
            .unwrap()
            .as_dict_mut()
            .unwrap()
            .set("Fm", forma);
        let bytes = salvar(&mut doc);
        for saida in saidas_da_primeira(&bytes) {
            assert!(contem_marca(&saida, b"MARCA-IMAGEM-A"));
            assert!(contem_marca(&saida, b"MARCA-IMAGEM-B"));
        }
    }

    #[test]
    fn formulario_com_filtro_que_nao_le_mantem_todos_os_recursos() {
        let mut ilegivel = formulario(b"nao e conteudo", None);
        ilegivel.dict.set("Filter", "DCTDecode");
        let (mut doc, _) = documento_compartilhado(b"q /Fm Do Q", vec![("Fm", ilegivel)]);
        let bytes = salvar(&mut doc);
        for saida in saidas_da_primeira(&bytes) {
            assert!(contem_marca(&saida, b"MARCA-IMAGEM-A"));
            assert!(contem_marca(&saida, b"MARCA-IMAGEM-B"));
        }
    }

    #[test]
    fn fonte_type3_com_recursos_tambem_faz_manter_os_recursos() {
        let (mut doc, p1) = documento_compartilhado(b"q Q", vec![]);
        let fonte = doc.add_object(
            dictionary! {"Type" => "Font", "Subtype" => "Type3", "Resources" => dictionary! {}},
        );
        recursos_da_pagina(&mut doc, p1).set("Font", dictionary! {"F1" => fonte});
        mantem_a_imagem_a_sem_do(&mut doc);
    }

    #[test]
    fn nomes_em_dois_fluxos_de_conteudo_sao_mantidos() {
        let extras = vec![("ImC", imagem(b"MARCA-IMAGEM-C"))];
        let (mut doc, p1) = documento_compartilhado(b"q Q", extras);
        let a = doc.add_object(lopdf::Stream::new(dictionary! {}, b"q /ImA Do".to_vec()));
        let b = doc.add_object(lopdf::Stream::new(dictionary! {}, b"Q /ImC Do".to_vec()));
        doc.get_object_mut(p1)
            .unwrap()
            .as_dict_mut()
            .unwrap()
            .set("Contents", vec![Object::Reference(a), Object::Reference(b)]);
        let bytes = salvar(&mut doc);
        for saida in saidas_da_primeira(&bytes) {
            assert!(contem_marca(&saida, b"MARCA-IMAGEM-A"));
            assert!(contem_marca(&saida, b"MARCA-IMAGEM-C"));
            assert!(!contem_marca(&saida, b"MARCA-IMAGEM-B"));
        }
    }

    /// Duas páginas desenham o mesmo formulário, que exige manter tudo (tem padrão nos recursos
    /// próprios): a segunda página, que reaproveita a análise guardada, também não é podada.
    #[test]
    fn formulario_que_exige_manter_tudo_vale_tambem_na_segunda_pagina() {
        let proprios = dictionary! {"Pattern" => dictionary! {}};
        let extras = vec![("Fm", formulario(b"q Q", Some(proprios)))];
        let (mut doc, p1) = documento_compartilhado(b"q /Fm Do Q", extras);
        let pai = doc
            .get_object(p1)
            .unwrap()
            .as_dict()
            .unwrap()
            .get(b"Parent")
            .unwrap()
            .as_reference()
            .unwrap();
        let kids = doc
            .get_object(pai)
            .unwrap()
            .as_dict()
            .unwrap()
            .get(b"Kids")
            .unwrap()
            .as_array()
            .unwrap()
            .clone();
        let p2 = kids[1].as_reference().unwrap();
        let c2 = doc.add_object(lopdf::Stream::new(dictionary! {}, b"q /Fm Do Q".to_vec()));
        doc.get_object_mut(p2)
            .unwrap()
            .as_dict_mut()
            .unwrap()
            .set("Contents", c2);
        let bytes = salvar(&mut doc);
        let origem = PdfOrigem::carregar(&bytes).unwrap();
        let todas = [
            Pagina {
                numero: Some(1),
                girar_180: false,
            },
            Pagina {
                numero: Some(2),
                girar_180: false,
            },
        ];
        let passada = Document::load_mem(&origem.gerar_passada(&todas, 1).unwrap()).unwrap();
        for pagina in passada.get_pages().values() {
            let dict = passada.get_object(*pagina).unwrap().as_dict().unwrap();
            let recursos = passada
                .dereference(dict.get(b"Resources").unwrap())
                .unwrap()
                .1
                .as_dict()
                .unwrap();
            let xobjetos = recursos.get(b"XObject").unwrap().as_dict().unwrap();
            assert!(xobjetos.has(b"ImA") && xobjetos.has(b"ImB"));
        }
    }

    fn nomes(bytes: &[u8]) -> Vec<String> {
        let mut visitas = usize::MAX;
        let mut lista: Vec<String> = nomes_nos_bytes(bytes, usize::MAX, &mut visitas)
            .unwrap()
            .into_iter()
            .map(|n| String::from_utf8_lossy(&n).into_owned())
            .collect();
        lista.sort();
        lista
    }

    #[test]
    fn varredura_de_nomes_acha_nomes_coladas_e_em_qualquer_contexto() {
        assert_eq!(nomes(b"/ImA/ImB"), ["ImA", "ImB"]);
        assert_eq!(nomes(b"[/ImA]"), ["ImA"]);
        assert_eq!(nomes(b"<</ImA 1>>"), ["ImA"]);
        assert_eq!(nomes(b"(/ImA) % /ImB\n"), ["ImA", "ImB"]);
        assert_eq!(nomes(b"/Im#41 /Im#20C"), ["Im C", "ImA"]);
    }

    #[test]
    fn varredura_de_nomes_com_cerquilha_invalida_ou_truncada() {
        // O trecho antes do `#` entra também, porque a leitura encerra o nome ali.
        assert_eq!(nomes(b"/ab#zz"), ["ab", "ab#zz"]);
        assert_eq!(nomes(b"/ab#4"), ["ab", "ab#4"]);
        assert_eq!(nomes(b"/ab#"), ["ab", "ab#"]);
    }

    #[test]
    fn varredura_de_nomes_respeita_os_tetos() {
        let mut visitas = usize::MAX;
        assert!(nomes_nos_bytes(b"/a /b /c", 2, &mut visitas).is_none());
        let mut visitas = 2;
        assert!(nomes_nos_bytes(b"/a /b /c", usize::MAX, &mut visitas).is_none());
        let mut visitas = 3;
        assert_eq!(
            nomes_nos_bytes(b"/a /b /c", 3, &mut visitas).unwrap().len(),
            3
        );
    }

    /// Nomes mantidos em `/XObject` por `recursos_podados` para a página 1 do documento.
    fn xobjetos_mantidos(
        doc: &Document,
        p1: ObjectId,
        conteudo: &[u8],
        analise: &mut AnaliseForms,
    ) -> Vec<String> {
        let pagina = pagina_com_heranca(doc, p1).unwrap();
        let recursos = pagina.get(b"Resources").unwrap().clone();
        let podados = recursos_podados(
            doc,
            &recursos,
            &pagina,
            Some(conteudo),
            &mut Orcamento::novo(),
            analise,
        )
        .unwrap();
        let (_, podados) = doc.dereference(&podados).unwrap();
        let mut lista: Vec<String> = podados
            .as_dict()
            .unwrap()
            .get(b"XObject")
            .unwrap()
            .as_dict()
            .unwrap()
            .iter()
            .map(|(n, _)| String::from_utf8_lossy(n).into_owned())
            .collect();
        lista.sort();
        lista
    }

    fn documento_de_tetos() -> (Document, ObjectId) {
        documento_compartilhado(b"q Q", vec![("ImC", imagem(b"MARCA-IMAGEM-C"))])
    }

    #[test]
    fn dentro_dos_tetos_so_o_usado_e_mantido() {
        let (doc, p1) = documento_de_tetos();
        let mut analise = AnaliseForms::default();
        assert_eq!(
            xobjetos_mantidos(&doc, p1, b"/ImA Do", &mut analise),
            ["ImA"]
        );
    }

    #[test]
    fn muitos_nomes_distintos_fazem_manter_tudo() {
        let (doc, p1) = documento_de_tetos();
        let mut analise = AnaliseForms::com_limites(usize::MAX, usize::MAX, 2);
        let mantidos = xobjetos_mantidos(&doc, p1, b"/ImA Do /x /y /z", &mut analise);
        assert_eq!(mantidos, ["ImA", "ImB", "ImC"]);
    }

    #[test]
    fn teto_de_visitas_faz_manter_tudo() {
        let (doc, p1) = documento_de_tetos();
        let mut analise = AnaliseForms::com_limites(usize::MAX, 1, usize::MAX);
        let mantidos = xobjetos_mantidos(&doc, p1, b"/ImA Do /x", &mut analise);
        assert_eq!(mantidos, ["ImA", "ImB", "ImC"]);
    }

    #[test]
    fn teto_de_bytes_da_analise_faz_manter_tudo() {
        let (doc, p1) = documento_de_tetos();
        let mut analise = AnaliseForms::com_limites(3, usize::MAX, usize::MAX);
        let mantidos = xobjetos_mantidos(&doc, p1, b"/ImA Do", &mut analise);
        assert_eq!(mantidos, ["ImA", "ImB", "ImC"]);
    }

    /// O formulário é decodificado e cobrado uma vez: com o orçamento zerado, a segunda
    /// análise vem do que ficou guardado.
    #[test]
    fn formulario_analisado_uma_vez_nao_e_cobrado_de_novo() {
        let extras = vec![("Fm", formulario(&vec![b' '; 4096], None))];
        let (doc, p1) = documento_compartilhado(b"q Q", extras);
        let pagina = pagina_com_heranca(&doc, p1).unwrap();
        let recursos = pagina.get(b"Resources").unwrap().clone();
        let mut analise = AnaliseForms::default();
        let mut orcamento = Orcamento { restante: 4096 };
        recursos_podados(
            &doc,
            &recursos,
            &pagina,
            Some(b"/Fm Do"),
            &mut orcamento,
            &mut analise,
        )
        .unwrap();
        assert_eq!(orcamento.restante, 0);
        let mut sem_orcamento = Orcamento { restante: 0 };
        recursos_podados(
            &doc,
            &recursos,
            &pagina,
            Some(b"/Fm Do"),
            &mut sem_orcamento,
            &mut analise,
        )
        .unwrap();
    }

    #[test]
    fn fluxo_de_objetos_que_expande_demais_e_recusado_na_abertura() {
        let mut doc = Document::with_version("1.5");
        let root = doc.new_object_id();
        let enchimento = Object::String(vec![b'a'; 70 << 20], lopdf::StringFormat::Literal);
        let p1 = doc.add_object(
            dictionary! {"Type" => "Page", "Parent" => root, "Enchimento" => enchimento},
        );
        doc.objects.insert(
            root,
            Object::Dictionary(dictionary! {
                "Type" => "Pages", "Count" => 1, "Kids" => vec![Object::Reference(p1)],
                "MediaBox" => vec![0.into(), 0.into(), 200.into(), 300.into()]
            }),
        );
        let catalogo = doc.add_object(dictionary! {"Type" => "Catalog", "Pages" => root});
        doc.trailer.set("Root", catalogo);
        let opcoes = lopdf::SaveOptions::builder()
            .use_object_streams(true)
            .use_xref_streams(true)
            .compression_level(6)
            .build();
        let mut bytes = Vec::new();
        doc.save_with_options(&mut bytes, opcoes).unwrap();
        assert!(bytes.len() < 4 << 20, "{}", bytes.len());
        // O modo leniente da leitura descarta o fluxo que passou do limite e o objeto que ele
        // guardava some; a abertura falha por falta desse objeto, sem expandir os 70 MiB.
        assert!(PdfOrigem::carregar(&bytes).is_err());
    }

    #[test]
    fn ciclo_de_formularios_termina_e_mantem_so_o_usado() {
        let extras = vec![
            ("F1", formulario(b"/F2 Do /ImA Do", None)),
            ("F2", formulario(b"/F1 Do", None)),
        ];
        let (mut doc, _) = documento_compartilhado(b"/F1 Do", extras);
        let bytes = salvar(&mut doc);
        for saida in saidas_da_primeira(&bytes) {
            assert!(contem_marca(&saida, b"MARCA-IMAGEM-A"));
            assert!(!contem_marca(&saida, b"MARCA-IMAGEM-B"));
        }
    }

    #[test]
    fn analise_de_conteudo_que_falha_mantem_todos_os_recursos() {
        let (mut doc, p1) = documento_compartilhado(b"q /ImA Do Q", vec![]);
        let c = doc.add_object(lopdf::Stream::new(
            dictionary! {"Filter" => "DCTDecode"},
            b"nao e conteudo".to_vec(),
        ));
        doc.get_object_mut(p1)
            .unwrap()
            .as_dict_mut()
            .unwrap()
            .set("Contents", c);
        let bytes = salvar(&mut doc);
        let origem = PdfOrigem::carregar(&bytes).unwrap();
        let passada = origem.gerar_passada(&primeira_pagina(), 1).unwrap();
        let saida = Document::load_mem(&passada).unwrap();
        assert!(contem_marca(&saida, b"MARCA-IMAGEM-A"));
        assert!(contem_marca(&saida, b"MARCA-IMAGEM-B"));
    }

    fn fluxo_comprimido(megabytes: usize) -> lopdf::Stream {
        let mut fluxo = lopdf::Stream::new(dictionary! {}, vec![b' '; megabytes << 20]);
        fluxo.compress().unwrap();
        fluxo
    }

    fn documento_com_conteudos(fluxos: Vec<lopdf::Stream>) -> Vec<u8> {
        let (mut doc, p1) = documento_compartilhado(b"q Q", vec![]);
        let ids: Vec<Object> = fluxos
            .into_iter()
            .map(|f| Object::Reference(doc.add_object(f)))
            .collect();
        doc.get_object_mut(p1)
            .unwrap()
            .as_dict_mut()
            .unwrap()
            .set("Contents", ids);
        salvar(&mut doc)
    }

    fn gerar_as_duas(bytes: &[u8]) -> Vec<Result<Vec<u8>, ErroPdf>> {
        let origem = PdfOrigem::carregar(bytes).unwrap();
        vec![
            origem.gerar_passada(&primeira_pagina(), 1),
            origem.gerar_folhas_finais(&[folha_de_teste(Some((1, false)), None)], 1),
        ]
    }

    #[test]
    fn bomba_de_compressao_em_um_fluxo_de_conteudo_e_recusada() {
        let bytes = documento_com_conteudos(vec![fluxo_comprimido(65)]);
        for resultado in gerar_as_duas(&bytes) {
            let erro = resultado.unwrap_err().to_string();
            assert!(erro.contains("limite"), "{erro}");
            assert!(erro.contains("fluxo"), "{erro}");
        }
    }

    #[test]
    fn conteudo_total_acima_do_limite_da_passada_e_recusado() {
        let um = fluxo_comprimido(52);
        let bytes =
            documento_com_conteudos(vec![um.clone(), um.clone(), um.clone(), um.clone(), um]);
        for resultado in gerar_as_duas(&bytes) {
            let erro = resultado.unwrap_err().to_string();
            assert!(erro.contains("limite"), "{erro}");
            assert!(erro.contains("total"), "{erro}");
            assert!(erro.contains("PDF de saída"), "{erro}");
        }
    }

    #[test]
    fn conteudo_grande_dentro_do_limite_e_aceito() {
        let bytes = documento_com_conteudos(vec![fluxo_comprimido(20)]);
        for resultado in gerar_as_duas(&bytes) {
            resultado.unwrap();
        }
    }

    #[test]
    fn formularios_aninhados_demais_sao_recusados_com_motivo() {
        let nomes: Vec<String> = (0..40).map(|i| format!("G{i}")).collect();
        let extras = nomes
            .iter()
            .enumerate()
            .map(|(i, nome)| {
                let conteudo = nomes
                    .get(i + 1)
                    .map(|proximo| format!("/{proximo} Do"))
                    .unwrap_or_default();
                (nome.as_str(), formulario(conteudo.as_bytes(), None))
            })
            .collect();
        let (mut doc, _) = documento_compartilhado(b"/G0 Do", extras);
        let bytes = salvar(&mut doc);
        let origem = PdfOrigem::carregar(&bytes).unwrap();
        let erro = origem
            .gerar_passada(&primeira_pagina(), 1)
            .unwrap_err()
            .to_string();
        assert!(erro.contains("aninhados"), "{erro}");
    }

    #[test]
    fn papel_vem_da_caixa_que_o_leitor_mostra() {
        let (mut doc, p1) = documento_compartilhado(b"q Q", vec![]);
        let pagina = doc.get_object_mut(p1).unwrap().as_dict_mut().unwrap();
        pagina.set("MediaBox", vec![0.into(), 0.into(), 700.into(), 900.into()]);
        pagina.set(
            "CropBox",
            vec![
                Object::Real(0.0),
                Object::Real(0.0),
                Object::Real(595.28),
                Object::Real(841.89),
            ],
        );
        let bytes = salvar(&mut doc);
        let origem = PdfOrigem::carregar(&bytes).unwrap();
        let dimensoes = origem.dimensoes_paginas().unwrap();
        assert!((dimensoes[0].largura_mm - 210.0).abs() < 0.1);
        assert!((dimensoes[0].altura_mm - 297.0).abs() < 0.1);
        // A página sem CropBox continua medida pela MediaBox.
        assert!((dimensoes[1].largura_mm - 200.0 * 25.4 / 72.0).abs() < 0.1);
    }
}

fn pagina_com_heranca(doc: &Document, id: ObjectId) -> Result<Dictionary, ErroPdf> {
    let mut folha = doc.get_object(id)?.as_dict()?.clone();
    let mut atual = id;
    let mut visitados = HashSet::new();
    // A árvore de páginas pode ser fornecida por um PDF hostil.
    for _ in 0..256 {
        if !visitados.insert(atual) {
            return Err(ErroPdf::DocumentoInvalido("ciclo na árvore de páginas"));
        }
        let dict = doc.get_object(atual)?.as_dict()?;
        for chave in [b"MediaBox".as_slice(), b"CropBox", b"Resources", b"Rotate"] {
            if !folha.has(chave) {
                if let Ok(valor) = dict.get(chave) {
                    folha.set(chave, valor.clone());
                }
            }
        }
        match dict.get(b"Parent") {
            Ok(parent) => atual = parent.as_reference()?,
            Err(_) => {
                let media = folha
                    .get(b"MediaBox")
                    .map_err(|_| ErroPdf::DocumentoInvalido("MediaBox ausente"))?;
                caixa(doc, media)?;
                if let Ok(crop) = folha.get(b"CropBox") {
                    caixa(doc, crop)?;
                }
                if let Ok(resources) = folha.get(b"Resources") {
                    doc.dereference(resources)?.1.as_dict()?;
                }
                return Ok(folha);
            }
        }
    }
    Err(ErroPdf::DocumentoInvalido(
        "árvore de páginas profunda demais",
    ))
}
