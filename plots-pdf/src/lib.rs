//! Extração de páginas para uma passada de impressão.

use std::collections::{HashMap, HashSet};

use lopdf::{dictionary, Dictionary, Document, Object, ObjectId};
use plots_core::{Pagina, LIMITE_PAGINAS};

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

/// Documento aberto sem senha. PDFs criptografados não são aceitos.
pub struct PdfOrigem {
    documento: Document,
    paginas: Vec<ObjectId>,
}

impl PdfOrigem {
    pub fn carregar(bytes: &[u8]) -> Result<Self, ErroPdf> {
        let documento = Document::load_mem(bytes)?;
        if documento.is_encrypted() || documento.trailer.has(b"Encrypt") {
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

    pub fn total_paginas(&self) -> usize {
        self.paginas.len()
    }

    /// Tamanho físico de cada página, em milímetros, considerada sua rotação.
    pub fn dimensoes_paginas(&self) -> Result<Vec<plots_core::DimensoesPagina>, ErroPdf> {
        let mut dimensoes = Vec::with_capacity(self.paginas.len());
        for id in &self.paginas {
            let dict = pagina_com_heranca(&self.documento, *id)?;
            let [x0, y0, x1, y1] = caixa(&self.documento, dict.get(b"MediaBox")?)?;
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
            let largura = f64::from(x1 - x0) * f64::from(unidade) * 25.4 / 72.0;
            let altura = f64::from(y1 - y0) * f64::from(unidade) * 25.4 / 72.0;
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
                let [mut x0, mut y0, mut x1, mut y1] = caixa(&self.documento, &media)?;
                if let Some(crop) = &crop {
                    let [cx0, cy0, cx1, cy1] = caixa(&self.documento, crop)?;
                    x0 = x0.max(cx0);
                    y0 = y0.max(cy0);
                    x1 = x1.min(cx1);
                    y1 = y1.min(cy1);
                    if x0 >= x1 || y0 >= y1 {
                        return Err(ErroPdf::DocumentoInvalido("CropBox fora de MediaBox"));
                    }
                }
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
                for chave in [b"DA".as_slice(), b"DR", b"Q", b"SigFlags"] {
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
        remover_inacessiveis(&mut saida, &proibidos, &paginas_novas)?;
        let mut bytes = Vec::new();
        saida.save_to(&mut bytes).map_err(ErroPdf::Escrita)?;
        Ok(bytes)
    }
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
        let widget = dict.get(b"Subtype").ok().and_then(|v| v.as_name().ok()) == Some(b"Widget");
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
                    "referência insegura à árvore de páginas original",
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
                        "objeto de página embutido inseguro",
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
