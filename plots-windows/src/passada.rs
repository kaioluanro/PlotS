//! O que o envio decide sobre uma passada antes de falar com o sistema.

use crate::erro::ErroEnvio;
use plots_core::{reconhecer_papel, DimensoesPagina, Pagina, Papel};
use plots_pdf::PdfOrigem;
use plots_print::job::Passada;
use std::ffi::OsStr;
use std::path::{Path, PathBuf};

/// Variável com a pasta da biblioteca de rasterização.
pub const VARIAVEL_PASTA_BIBLIOTECA: &str = "PLOTS_PDFIUM_DIR";
const NOME_BIBLIOTECA: &str = "pdfium.dll";

/// Todo PDF passa por esta barreira antes da biblioteca de rasterização. Ela recusa PDF
/// protegido, inclusive o que só tem senha de dono e abriria com a senha vazia; nenhuma senha é
/// pedida ou informada por este programa e nenhuma permissão é ignorada.
pub fn abrir_origem(bytes: &[u8]) -> Result<PdfOrigem, ErroEnvio> {
    PdfOrigem::carregar(bytes).map_err(|erro| ErroEnvio::Pdf(erro.to_string()))
}

/// Nome do documento na fila: nunca leva caminho nem nome de arquivo do usuário.
pub fn nome_documento(lote: usize, passada: Passada) -> String {
    format!("PlotS - lote {lote} - {}", nome_da_passada(passada))
}

/// Nome do arquivo gravado quando o envio tem pasta de saída. O conteúdo é o que o driver da
/// fila produz: só a porta que pede arquivo é a dos drivers que geram um documento, e mesmo ali a
/// extensão é só um rótulo, porque o formato depende do driver. Nas demais, sai `.saida`.
pub fn nome_arquivo_saida(lote: usize, passada: Passada, porta: &str) -> String {
    let extensao = if porta_pede_arquivo(porta) {
        "pdf"
    } else {
        "saida"
    };
    format!("lote-{lote:04}-{}.{extensao}", nome_da_passada(passada))
}

fn nome_da_passada(passada: Passada) -> &'static str {
    match passada {
        Passada::Frente => "frente",
        Passada::Verso => "verso",
    }
}

/// Papel de uma passada, lido da primeira página dela. A página em branco só existe no fim de
/// um documento ímpar e tem o tamanho da última página, a mesma regra de `gerar_passada`.
pub fn papel_da_passada(
    paginas: &[Pagina],
    dimensoes: &[DimensoesPagina],
) -> Result<Papel, ErroEnvio> {
    let primeira = paginas.first().ok_or(ErroEnvio::PassadaVazia)?;
    let numero = primeira.numero.unwrap_or(dimensoes.len());
    let medida = numero
        .checked_sub(1)
        .and_then(|indice| dimensoes.get(indice))
        .ok_or(ErroEnvio::PaginaInexistente(numero))?;
    reconhecer_papel(*medida).map_err(|_| ErroEnvio::TamanhoInvalido)
}

/// Caminhos aceitos para a biblioteca, em ordem: a pasta indicada (direto ou em `bin`, como o
/// script a extrai) ou, sem indicação, ao lado do executável. Nunca a busca do sistema.
pub fn caminhos_da_biblioteca(
    pasta_indicada: Option<&OsStr>,
    pasta_do_executavel: Option<&Path>,
) -> Result<Vec<PathBuf>, ErroEnvio> {
    if let Some(pasta) = pasta_indicada.filter(|pasta| !pasta.is_empty()) {
        let pasta = Path::new(pasta);
        if !pasta.is_absolute() {
            return Err(ErroEnvio::BibliotecaCaminhoRelativo(pasta.to_path_buf()));
        }
        return Ok(vec![
            pasta.join(NOME_BIBLIOTECA),
            pasta.join("bin").join(NOME_BIBLIOTECA),
        ]);
    }
    Ok(pasta_do_executavel
        .map(|pasta| pasta.join(NOME_BIBLIOTECA))
        .into_iter()
        .collect())
}

/// Primeiro caminho que existe, ou o erro que aponta o script.
pub fn escolher_biblioteca(
    caminhos: Vec<PathBuf>,
    existe: impl Fn(&Path) -> bool,
) -> Result<PathBuf, ErroEnvio> {
    match caminhos.iter().find(|caminho| existe(caminho)) {
        Some(caminho) => Ok(caminho.clone()),
        None => Err(ErroEnvio::BibliotecaAusente {
            procurados: caminhos,
        }),
    }
}

/// Portas em que o Windows pergunta o nome do arquivo em um diálogo. Mandar para elas sem um
/// arquivo de saída travaria o envio esperando alguém responder.
pub fn porta_pede_arquivo(porta: &str) -> bool {
    porta.split(',').map(str::trim).any(|parte| {
        parte.eq_ignore_ascii_case("PORTPROMPT:") || parte.eq_ignore_ascii_case("FILE:")
    })
}

/// Porta de um fax compartilhado: mandar para ela discaria em vez de imprimir.
pub fn porta_de_fax(porta: &str) -> bool {
    porta
        .split(',')
        .map(str::trim)
        .any(|parte| parte.eq_ignore_ascii_case("SHRFAX:"))
}

#[cfg(test)]
mod testes {
    use super::*;

    const A4: DimensoesPagina = DimensoesPagina::new(210.0, 297.0);
    const A5: DimensoesPagina = DimensoesPagina::new(148.0, 210.0);

    fn pagina(numero: Option<usize>) -> Pagina {
        Pagina {
            numero,
            girar_180: false,
        }
    }

    /// PDF de uma página, criptografado com as senhas e permissões dadas.
    fn pdf_protegido_com(usuario: &str, permissoes: lopdf::Permissions) -> Vec<u8> {
        use lopdf::{dictionary, Document, Object};
        let mut doc = Document::with_version("1.5");
        let raiz = doc.new_object_id();
        let folha = doc.add_object(dictionary! {
            "Type" => "Page", "Parent" => raiz,
            "MediaBox" => vec![0.into(), 0.into(), 595.into(), 842.into()],
            "Resources" => dictionary! {}
        });
        doc.objects.insert(
            raiz,
            Object::Dictionary(dictionary! {
                "Type" => "Pages", "Count" => 1, "Kids" => vec![Object::Reference(folha)]
            }),
        );
        let catalogo = doc.add_object(dictionary! {"Type" => "Catalog", "Pages" => raiz});
        doc.trailer.set("Root", catalogo);
        doc.trailer.set(
            "ID",
            vec![
                Object::string_literal("id-de-teste"),
                Object::string_literal("id-de-teste"),
            ],
        );
        let versao = lopdf::EncryptionVersion::V1 {
            document: &doc,
            owner_password: "dono",
            user_password: usuario,
            permissions: permissoes,
        };
        let estado = lopdf::EncryptionState::try_from(versao).unwrap();
        doc.encrypt(&estado).unwrap();
        let mut bytes = Vec::new();
        doc.save_to(&mut bytes).unwrap();
        bytes
    }

    #[test]
    fn pdf_protegido_para_na_barreira() {
        let pdf = pdf_protegido_com("segredo", lopdf::Permissions::default());
        let erro = abrir_origem(&pdf).err().unwrap();
        assert!(erro.to_string().contains("criptografado"), "{erro}");
    }

    #[test]
    fn pdf_so_com_senha_de_dono_e_sem_permissoes_para_na_barreira() {
        let pdf = pdf_protegido_com("", lopdf::Permissions::empty());
        let erro = abrir_origem(&pdf).err().unwrap();
        assert!(erro.to_string().contains("criptografado"), "{erro}");
    }

    #[test]
    fn bytes_que_nao_sao_pdf_param_na_barreira() {
        assert!(matches!(
            abrir_origem(b"nada a ver").err().unwrap(),
            ErroEnvio::Pdf(_)
        ));
    }

    #[test]
    fn nome_do_documento_nao_leva_caminho() {
        let nome = nome_documento(3, Passada::Verso);
        assert_eq!(nome, "PlotS - lote 3 - verso");
        assert!(!nome.contains(['/', '\\', ':']));
    }

    #[test]
    fn extensao_do_arquivo_de_saida_depende_da_porta() {
        assert_eq!(
            nome_arquivo_saida(12, Passada::Frente, "PORTPROMPT:"),
            "lote-0012-frente.pdf"
        );
        assert_eq!(
            nome_arquivo_saida(12, Passada::Verso, "USB001"),
            "lote-0012-verso.saida"
        );
        assert_eq!(
            nome_arquivo_saida(1, Passada::Frente, ""),
            "lote-0001-frente.saida"
        );
    }

    #[test]
    fn porta_de_fax_e_reconhecida_em_qualquer_caixa_e_na_lista() {
        assert!(porta_de_fax("SHRFAX:"));
        assert!(porta_de_fax("shrfax:"));
        assert!(porta_de_fax("USB001, SHRFAX:"));
        assert!(!porta_de_fax("USB001"));
        assert!(!porta_de_fax("PORTPROMPT:"));
    }

    #[test]
    fn papel_vem_da_primeira_pagina_da_passada() {
        let dimensoes = [A4, A4, A5, A5];
        let papel = papel_da_passada(&[pagina(Some(3))], &dimensoes).unwrap();
        assert_eq!(papel.formato, plots_core::FormatoPapel::A5);
    }

    #[test]
    fn pagina_em_branco_usa_o_tamanho_da_ultima_pagina() {
        let dimensoes = [A4, A4, A5];
        let papel = papel_da_passada(&[pagina(None)], &dimensoes).unwrap();
        assert_eq!(papel.formato, plots_core::FormatoPapel::A5);
    }

    #[test]
    fn passada_vazia_ou_pagina_fora_do_documento_viram_erro() {
        assert_eq!(
            papel_da_passada(&[], &[A4]).unwrap_err(),
            ErroEnvio::PassadaVazia
        );
        assert_eq!(
            papel_da_passada(&[pagina(Some(9))], &[A4]).unwrap_err(),
            ErroEnvio::PaginaInexistente(9)
        );
        assert_eq!(
            papel_da_passada(&[pagina(Some(0))], &[A4]).unwrap_err(),
            ErroEnvio::PaginaInexistente(0)
        );
        assert_eq!(
            papel_da_passada(&[pagina(None)], &[]).unwrap_err(),
            ErroEnvio::PaginaInexistente(0)
        );
    }

    #[test]
    fn pasta_indicada_vence_e_nunca_usa_a_busca_do_sistema() {
        let indicada = if cfg!(windows) { "C:\\dll" } else { "/dll" };
        let exe = Path::new("exe");
        let caminhos = caminhos_da_biblioteca(Some(OsStr::new(indicada)), Some(exe)).unwrap();
        assert_eq!(caminhos.len(), 2);
        assert!(caminhos.iter().all(|c| c.is_absolute()));
        assert!(caminhos.iter().all(|c| c.ends_with("pdfium.dll")));
        assert!(caminhos[1].ends_with("bin/pdfium.dll"));
    }

    #[test]
    fn sem_indicacao_procura_so_ao_lado_do_executavel() {
        let exe = Path::new("exe");
        assert_eq!(
            caminhos_da_biblioteca(None, Some(exe)).unwrap(),
            vec![exe.join("pdfium.dll")]
        );
        assert_eq!(
            caminhos_da_biblioteca(Some(OsStr::new("")), Some(exe)).unwrap(),
            vec![exe.join("pdfium.dll")]
        );
        assert!(caminhos_da_biblioteca(None, None).unwrap().is_empty());
    }

    #[test]
    fn pasta_relativa_e_recusada() {
        assert!(matches!(
            caminhos_da_biblioteca(Some(OsStr::new("relativa")), None).unwrap_err(),
            ErroEnvio::BibliotecaCaminhoRelativo(_)
        ));
    }

    #[test]
    fn sem_a_biblioteca_o_erro_aponta_o_script() {
        let erro = escolher_biblioteca(vec![PathBuf::from("a/pdfium.dll")], |_| false).unwrap_err();
        assert!(erro.to_string().contains("baixar-pdfium"), "{erro}");
        let existe = escolher_biblioteca(vec![PathBuf::from("a"), PathBuf::from("b")], |caminho| {
            caminho == Path::new("b")
        });
        assert_eq!(existe.unwrap(), PathBuf::from("b"));
    }

    #[test]
    fn portas_que_perguntam_o_arquivo_sao_reconhecidas() {
        assert!(porta_pede_arquivo("PORTPROMPT:"));
        assert!(porta_pede_arquivo("portprompt:"));
        assert!(porta_pede_arquivo("FILE:"));
        assert!(porta_pede_arquivo("USB001, FILE:"));
        assert!(!porta_pede_arquivo("USB001"));
        assert!(!porta_pede_arquivo("192.168.0.2"));
        assert!(!porta_pede_arquivo(""));
    }
}
