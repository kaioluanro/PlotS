//! Geração das passadas em PDF, compartilhada por `gerar-pdf`, `imprimir` e a calibração.

use crate::{GESTO_CORRETO, MSG_AVISO_IMPARES};
use plots_core::{
    pares_com_orientacao_mista, plano_lotes_por_papel, DimensoesPagina, Lote, LotePapel,
    OrdemPassadas, Pagina, Papel, PerfilImpressora,
};
use plots_pdf::{FaceFinal, FolhaFinal, PdfOrigem};
use plots_print::{Marca, Mecanica, Simulador};
use std::io::Write;
use std::path::{Path, PathBuf};

const AVISO_ASSINATURA: &str = "Aviso: este PDF tem assinatura digital. As cópias geradas servem só para imprimir: mostram a aparência da assinatura, mas não a contêm. Para enviar ou guardar o documento, use o arquivo original.";

/// Documento de entrada já lido, com o que a geração e os avisos precisam.
pub(crate) struct Documento {
    pdf: PdfOrigem,
    assinado: bool,
    dimensoes: Vec<DimensoesPagina>,
}

impl Documento {
    /// Lê o PDF do disco e reúne as dimensões das páginas e a presença de assinatura.
    pub(crate) fn ler(caminho: &Path) -> Result<Self, String> {
        let bytes =
            std::fs::read(caminho).map_err(|e| format!("não foi possível ler o PDF: {e}"))?;
        let pdf = PdfOrigem::carregar(&bytes).map_err(|e| e.to_string())?;
        let assinado = pdf.tem_assinatura_aplicada().map_err(|e| e.to_string())?;
        let dimensoes = pdf.dimensoes_paginas().map_err(|e| e.to_string())?;
        Ok(Self {
            pdf,
            assinado,
            dimensoes,
        })
    }

    /// Quantidade de páginas do documento.
    pub(crate) fn paginas(&self) -> usize {
        self.pdf.total_paginas()
    }

    /// Devolve o PDF já aberto e as dimensões das páginas, para o envio real.
    #[cfg(windows)]
    pub(crate) fn em_partes(self) -> (PdfOrigem, Vec<DimensoesPagina>) {
        (self.pdf, self.dimensoes)
    }
}

/// Parâmetros de planejamento e de geração de um documento.
pub(crate) struct Opcoes {
    pub tamanho: usize,
    pub ordem: OrdemPassadas,
    pub perfil: PerfilImpressora,
    pub mecanica: Mecanica,
    pub folhas_finais: bool,
}

/// Lotes planejados e os pares de páginas com orientação mista.
pub(crate) struct Plano {
    pub lotes: Vec<LotePapel>,
    pub pares_mistos: Vec<(usize, usize)>,
}

/// Valida o documento e planeja os lotes antes de qualquer escrita.
pub(crate) fn planejar(
    documento: &Documento,
    tamanho: usize,
    ordem: OrdemPassadas,
    perfil: PerfilImpressora,
) -> Result<Plano, String> {
    let lotes = plano_lotes_por_papel(&documento.dimensoes, tamanho, ordem, perfil)
        .map_err(|e| e.to_string())?;
    let pares_mistos =
        pares_com_orientacao_mista(&documento.dimensoes).map_err(|e| e.to_string())?;
    Ok(Plano {
        lotes,
        pares_mistos,
    })
}

/// Um lote na forma que o controle de trabalhos usa, com o papel em que ele sai.
#[derive(Clone)]
pub(crate) struct LoteDoPlano {
    pub papel: Papel,
    pub lote: Lote,
}

/// Lotes do plano, cada um com o seu papel.
pub(crate) fn lotes_do_plano(plano: &Plano) -> Vec<LoteDoPlano> {
    plano
        .lotes
        .iter()
        .map(|lote| LoteDoPlano {
            papel: lote.papel,
            lote: Lote {
                paginas: lote
                    .paginas
                    .iter()
                    .map(|numero| Pagina {
                        numero: *numero,
                        girar_180: false,
                    })
                    .collect(),
                passada1: lote.passada1.clone(),
                passada2: lote.passada2.clone(),
            },
        })
        .collect()
}

/// Gera os PDFs de cada lote em memória, com os nomes que `gerar-pdf` usa.
pub(crate) fn gerar_arquivos(
    documento: &Documento,
    plano: &Plano,
    opcoes: &Opcoes,
    diretorio: &Path,
) -> Result<Vec<(PathBuf, Vec<u8>)>, String> {
    let mut arquivos = Vec::new();
    for (i, lote) in plano.lotes.iter().enumerate() {
        let referencia = lote
            .paginas
            .iter()
            .flatten()
            .last()
            .copied()
            .ok_or("lote vazio")?;
        for (passada, paginas) in [(1, &lote.passada1), (2, &lote.passada2)] {
            let caminho = diretorio.join(format!("lote-{:04}-passada-{passada}.pdf", i + 1));
            let conteudo = documento
                .pdf
                .gerar_passada(paginas, referencia)
                .map_err(|e| e.to_string())?;
            arquivos.push((caminho, conteudo));
        }
        if opcoes.folhas_finais {
            let folhas = folhas_finais_do_lote(&lote.passada1, &lote.passada2, opcoes.mecanica)?;
            let conteudo = documento
                .pdf
                .gerar_folhas_finais(&folhas, referencia)
                .map_err(|e| e.to_string())?;
            let caminho = diretorio.join(format!("lote-{:04}-folhas-finais.pdf", i + 1));
            arquivos.push((caminho, conteudo));
        }
    }
    Ok(arquivos)
}

/// Grava um arquivo novo; nunca sobrescreve um existente.
pub(crate) fn criar_novo(caminho: &Path, conteudo: &[u8]) -> Result<(), String> {
    gravar_sem_sobrescrever(&[(caminho.to_path_buf(), conteudo.to_vec())])
}

/// Grava todos os arquivos ou nenhum: se um falhar, os criados nesta chamada são removidos.
/// Arquivos que já existiam nunca são tocados.
pub(crate) fn gravar_sem_sobrescrever(arquivos: &[(PathBuf, Vec<u8>)]) -> Result<(), String> {
    let mut criados = Vec::new();
    for (caminho, conteudo) in arquivos {
        if let Err(erro) = criar_um(caminho, conteudo, &mut criados) {
            for criado in &criados {
                let _ = std::fs::remove_file(criado);
            }
            return Err(erro);
        }
    }
    Ok(())
}

fn criar_um(caminho: &Path, conteudo: &[u8], criados: &mut Vec<PathBuf>) -> Result<(), String> {
    use std::io::ErrorKind;
    let mut arquivo = std::fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(caminho)
        .map_err(|e| {
            if e.kind() == ErrorKind::AlreadyExists {
                format!(
                    "o arquivo {} já existe; escolha outra pasta ou remova o arquivo existente",
                    caminho.display()
                )
            } else {
                format!("não foi possível criar {}: {e}", caminho.display())
            }
        })?;
    criados.push(caminho.to_path_buf());
    arquivo
        .write_all(conteudo)
        .map_err(|e| format!("não foi possível gravar {}: {e}", caminho.display()))
}

/// Avisos do documento: folhas com orientação mista, ordem invertida e assinatura.
pub(crate) fn escrever_avisos(
    documento: &Documento,
    plano: &Plano,
    ordem: OrdemPassadas,
    saida: &mut impl Write,
) -> Result<(), String> {
    for (frente, verso) in &plano.pares_mistos {
        writeln!(
            saida,
            "Aviso: a folha das páginas {frente} e {verso} mistura retrato e paisagem; confira o resultado antes de imprimir."
        )
        .map_err(|e| e.to_string())?;
    }
    if ordem == OrdemPassadas::ImparesPrimeiro {
        writeln!(saida, "{MSG_AVISO_IMPARES}").map_err(|e| e.to_string())?;
    }
    if documento.assinado {
        writeln!(saida, "{AVISO_ASSINATURA}").map_err(|e| e.to_string())?;
    }
    Ok(())
}

/// Simula um lote com a recolocação correta e devolve as folhas finais, de cima para baixo.
fn folhas_finais_do_lote(
    frente: &[plots_core::Pagina],
    verso: &[plots_core::Pagina],
    mecanica: Mecanica,
) -> Result<Vec<FolhaFinal>, String> {
    let mut sim = Simulador::novo(mecanica);
    sim.imprimir_frente(frente)
        .map_err(|e| format!("erro na frente: {e}"))?;
    sim.recolocar(GESTO_CORRETO)
        .map_err(|e| format!("erro na recolocação: {e}"))?;
    sim.imprimir_verso(verso)
        .map_err(|e| format!("erro no verso: {e}"))?;
    Ok(sim
        .folhas_para_leitura()
        .iter()
        .rev()
        .map(|folha| ordenar_lados(folha.face_a.map(face_final), folha.face_b.map(face_final)))
        .collect())
}

/// Escolhe o lado de cada face: a página de número menor fica à esquerda e a face em branco
/// vai por último, à direita. A orientação de cada face acompanha a própria face.
fn ordenar_lados(a: Option<FaceFinal>, b: Option<FaceFinal>) -> FolhaFinal {
    let chave = |face: &Option<FaceFinal>| face.and_then(|f| f.pagina).map_or((1, 0), |n| (0, n));
    if chave(&b) < chave(&a) {
        FolhaFinal {
            frente: b,
            verso: a,
        }
    } else {
        FolhaFinal {
            frente: a,
            verso: b,
        }
    }
}

fn face_final(marca: Marca) -> FaceFinal {
    FaceFinal {
        pagina: marca.pagina,
        girada: marca.girada,
    }
}
