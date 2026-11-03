//! Validacao do caminho de um documento antes de ele virar uma aba.

use plots_pdf::{ErroPdf, PdfOrigem};
use serde::Serialize;
use std::fs::File;
use std::io::{ErrorKind, Read};
use std::path::Path;
use std::sync::{Mutex, PoisonError};
use tauri::ipc::Response;

/// Documento aceito para abrir em uma aba.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct Documento {
    pub caminho: String,
    pub nome: String,
}

/// Motivo da recusa. A interface traduz o codigo; o nome identifica o arquivo na mensagem.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum CodigoErro {
    NaoEncontrado,
    NaoEArquivo,
    NaoEPdf,
    Inacessivel,
    PdfInvalido,
    Criptografado,
    Grande,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct ErroDocumento {
    pub codigo: CodigoErro,
    pub nome: String,
}

/// Maior PDF aceito para leitura (1 GiB). Um arquivo maior e recusado antes de ocupar a memoria.
pub const LIMITE_BYTES_PDF: u64 = 1024 * 1024 * 1024;

/// Aceita o caminho quando ele existe, e um arquivo e tem extensao `.pdf`, em qualquer caixa.
pub fn abrir(caminho: &str) -> Result<Documento, ErroDocumento> {
    let alvo = Path::new(caminho);
    let nome = nome_de_exibicao(caminho);
    let erro = |codigo| ErroDocumento {
        codigo,
        nome: nome.clone(),
    };

    let metadados = std::fs::metadata(alvo).map_err(|falha| {
        erro(match falha.kind() {
            ErrorKind::NotFound => CodigoErro::NaoEncontrado,
            _ => CodigoErro::Inacessivel,
        })
    })?;
    if !metadados.is_file() {
        return Err(erro(CodigoErro::NaoEArquivo));
    }
    let eh_pdf = alvo
        .extension()
        .is_some_and(|extensao| extensao.eq_ignore_ascii_case("pdf"));
    if !eh_pdf {
        return Err(erro(CodigoErro::NaoEPdf));
    }

    Ok(Documento {
        caminho: caminho.to_owned(),
        nome: nome.clone(),
    })
}

/// Total de paginas, depois de o arquivo passar pela mesma barreira que a impressao usa.
pub fn contar_paginas(caminho: &str) -> Result<usize, ErroDocumento> {
    contar_com_limite(caminho, LIMITE_BYTES_PDF)
}

fn contar_com_limite(caminho: &str, limite: u64) -> Result<usize, ErroDocumento> {
    carregar_validado(caminho, limite).map(|(_, total)| total)
}

/// Bytes do PDF para a interface mostrar, sempre depois da mesma barreira da impressao. Um PDF
/// recusado aqui nunca chega ao leitor da interface, que nao recebe senha nem abre PDF protegido.
///
/// Devolve os bytes crus, sem passar por JSON. A leitura roda em uma thread propria, e o que a
/// interromper vira recusa.
#[tauri::command]
pub async fn ler_documento(caminho: String) -> Result<Response, ErroDocumento> {
    let falha = falha_inesperada(&caminho);
    tauri::async_runtime::spawn_blocking(move || ler_validado(&caminho))
        .await
        .unwrap_or(Err(falha))
        .map(Response::new)
}

fn ler_validado(caminho: &str) -> Result<Vec<u8>, ErroDocumento> {
    ler_com_limite_validado(caminho, LIMITE_BYTES_PDF)
}

fn ler_com_limite_validado(caminho: &str, limite: u64) -> Result<Vec<u8>, ErroDocumento> {
    carregar_validado(caminho, limite).map(|(bytes, _)| bytes)
}

/// Um PDF por vez: a contagem e a leitura de varias abas nao disputam a memoria ao mesmo tempo.
static CARGA_EM_ANDAMENTO: Mutex<()> = Mutex::new(());

fn carregar_validado(caminho: &str, limite: u64) -> Result<(Vec<u8>, usize), ErroDocumento> {
    let documento = abrir(caminho)?;
    let erro = |codigo| ErroDocumento {
        codigo,
        nome: documento.nome.clone(),
    };
    let _vez = CARGA_EM_ANDAMENTO
        .lock()
        .unwrap_or_else(PoisonError::into_inner);
    let bytes = ler_com_limite(Path::new(caminho), limite).map_err(erro)?;
    let total = PdfOrigem::carregar(&bytes)
        .map_err(|falha| {
            erro(match falha {
                ErroPdf::Criptografado => CodigoErro::Criptografado,
                _ => CodigoErro::PdfInvalido,
            })
        })?
        .total_paginas();
    Ok((bytes, total))
}

/// Le o arquivo sem passar do limite, mesmo que ele cresca depois de medido. Faltar memoria para o
/// conteudo vira recusa em vez de encerrar o programa.
fn ler_com_limite(alvo: &Path, limite: u64) -> Result<Vec<u8>, CodigoErro> {
    let de_leitura = |falha: std::io::Error| match falha.kind() {
        ErrorKind::NotFound => CodigoErro::NaoEncontrado,
        _ => CodigoErro::Inacessivel,
    };
    let arquivo = File::open(alvo).map_err(de_leitura)?;
    let tamanho = arquivo.metadata().map_err(de_leitura)?.len();
    if tamanho > limite {
        return Err(CodigoErro::Grande);
    }
    let capacidade = usize::try_from(tamanho).map_err(|_| CodigoErro::Grande)?;
    let mut bytes = Vec::new();
    bytes
        .try_reserve_exact(capacidade)
        .map_err(|_| CodigoErro::Grande)?;
    arquivo
        .take(limite.saturating_add(1))
        .read_to_end(&mut bytes)
        .map_err(de_leitura)?;
    let lidos = u64::try_from(bytes.len()).map_err(|_| CodigoErro::Grande)?;
    if lidos > limite {
        return Err(CodigoErro::Grande);
    }
    Ok(bytes)
}

/// Recusa para quando a propria contagem nao chega ao fim.
pub fn falha_inesperada(caminho: &str) -> ErroDocumento {
    ErroDocumento {
        codigo: CodigoErro::PdfInvalido,
        nome: nome_de_exibicao(caminho),
    }
}

/// Ultimo componente do caminho; sem ele (raiz de disco, texto vazio) devolve o caminho inteiro.
fn nome_de_exibicao(caminho: &str) -> String {
    Path::new(caminho).file_name().map_or_else(
        || caminho.to_owned(),
        |nome| nome.to_string_lossy().into_owned(),
    )
}

#[cfg(test)]
mod testes {
    use super::*;
    use std::fs;
    use std::path::PathBuf;
    use std::sync::atomic::{AtomicU32, Ordering};

    static CONTADOR: AtomicU32 = AtomicU32::new(0);

    /// Pasta temporaria exclusiva do teste, removida ao sair de escopo.
    struct PastaTemporaria(PathBuf);

    impl PastaTemporaria {
        fn nova() -> Self {
            let numero = CONTADOR.fetch_add(1, Ordering::Relaxed);
            let pasta = std::env::temp_dir().join(format!(
                "plots-app-documento-{}-{numero}",
                std::process::id()
            ));
            fs::create_dir_all(&pasta).expect("criar pasta temporaria");
            Self(pasta)
        }

        fn arquivo(&self, nome: &str) -> String {
            let caminho = self.0.join(nome);
            fs::write(&caminho, b"%PDF-1.7\n").expect("criar arquivo");
            caminho.to_string_lossy().into_owned()
        }
    }

    impl Drop for PastaTemporaria {
        fn drop(&mut self) {
            let _ = fs::remove_dir_all(&self.0);
        }
    }

    #[test]
    fn aceita_arquivo_pdf_e_devolve_o_nome_de_exibicao() {
        let pasta = PastaTemporaria::nova();
        let caminho = pasta.arquivo("relatorio.pdf");
        assert_eq!(
            abrir(&caminho),
            Ok(Documento {
                caminho: caminho.clone(),
                nome: "relatorio.pdf".to_owned(),
            })
        );
    }

    #[test]
    fn aceita_extensao_em_maiusculas() {
        let pasta = PastaTemporaria::nova();
        let caminho = pasta.arquivo("SCAN.PDF");
        assert_eq!(
            abrir(&caminho).map(|doc| doc.nome),
            Ok("SCAN.PDF".to_owned())
        );
    }

    #[test]
    fn aceita_caminho_com_acento_e_espaco() {
        let pasta = PastaTemporaria::nova();
        let caminho = pasta.arquivo("Relatório de produção – março.pdf");
        assert_eq!(
            abrir(&caminho).map(|doc| doc.nome),
            Ok("Relatório de produção – março.pdf".to_owned())
        );
    }

    #[test]
    fn recusa_arquivo_inexistente() {
        let pasta = PastaTemporaria::nova();
        let caminho = pasta.0.join("sumiu.pdf");
        assert_eq!(
            abrir(&caminho.to_string_lossy()),
            Err(ErroDocumento {
                codigo: CodigoErro::NaoEncontrado,
                nome: "sumiu.pdf".to_owned(),
            })
        );
    }

    #[test]
    fn recusa_pasta_mesmo_com_nome_terminado_em_pdf() {
        let pasta = PastaTemporaria::nova();
        let subpasta = pasta.0.join("capitulos.pdf");
        fs::create_dir_all(&subpasta).expect("criar subpasta");
        assert_eq!(
            abrir(&subpasta.to_string_lossy()),
            Err(ErroDocumento {
                codigo: CodigoErro::NaoEArquivo,
                nome: "capitulos.pdf".to_owned(),
            })
        );
    }

    #[test]
    fn recusa_outra_extensao_e_arquivo_sem_extensao() {
        let pasta = PastaTemporaria::nova();
        for nome in ["notas.txt", "pdf", "falso.pdf.exe"] {
            let caminho = pasta.arquivo(nome);
            assert_eq!(
                abrir(&caminho),
                Err(ErroDocumento {
                    codigo: CodigoErro::NaoEPdf,
                    nome: nome.to_owned(),
                })
            );
        }
    }

    #[test]
    fn caminho_vazio_ou_invalido_vira_erro_sem_derrubar_o_app() {
        assert_eq!(
            abrir("").map_err(|erro| erro.codigo),
            Err(CodigoErro::NaoEncontrado)
        );
        assert_eq!(
            abrir("a\0b.pdf").map_err(|erro| erro.codigo),
            Err(CodigoErro::Inacessivel)
        );
    }

    /// PDF escrito byte a byte, com a tabela de referencias certa. `extra` entra como objetos
    /// adicionais antes da tabela e `trailer` dentro do dicionario final.
    fn pdf(paginas: usize, extra: &[&str], trailer: &str) -> Vec<u8> {
        let filhos: Vec<String> = (0..paginas).map(|n| format!("{} 0 R", n + 3)).collect();
        let mut objetos = vec![
            "<< /Type /Catalog /Pages 2 0 R >>".to_owned(),
            format!(
                "<< /Type /Pages /Count {paginas} /Kids [{}] >>",
                filhos.join(" ")
            ),
        ];
        for _ in 0..paginas {
            objetos.push("<< /Type /Page /Parent 2 0 R /MediaBox [0 0 595 842] >>".to_owned());
        }
        objetos.extend(extra.iter().map(|objeto| (*objeto).to_owned()));

        let mut bytes = b"%PDF-1.5\n".to_vec();
        let mut posicoes = Vec::new();
        for (indice, objeto) in objetos.iter().enumerate() {
            posicoes.push(bytes.len());
            bytes.extend_from_slice(format!("{} 0 obj\n{objeto}\nendobj\n", indice + 1).as_bytes());
        }
        let tabela = bytes.len();
        let total = objetos.len() + 1;
        bytes.extend_from_slice(format!("xref\n0 {total}\n0000000000 65535 f \n").as_bytes());
        for posicao in posicoes {
            bytes.extend_from_slice(format!("{posicao:010} 00000 n \n").as_bytes());
        }
        bytes.extend_from_slice(
            format!(
                "trailer\n<< /Size {total} /Root 1 0 R {trailer} >>\nstartxref\n{tabela}\n%%EOF\n"
            )
            .as_bytes(),
        );
        bytes
    }

    fn gravar(pasta: &PastaTemporaria, nome: &str, bytes: &[u8]) -> String {
        let caminho = pasta.0.join(nome);
        fs::write(&caminho, bytes).expect("gravar pdf");
        caminho.to_string_lossy().into_owned()
    }

    fn codigo_ao_contar(caminho: &str) -> Option<CodigoErro> {
        contar_paginas(caminho).err().map(|erro| erro.codigo)
    }

    #[test]
    fn conta_as_paginas_de_um_pdf_valido() {
        let pasta = PastaTemporaria::nova();
        for total in [1, 2, 24] {
            let caminho = gravar(&pasta, &format!("{total}.pdf"), &pdf(total, &[], ""));
            assert_eq!(contar_paginas(&caminho), Ok(total));
        }
    }

    #[test]
    fn recusa_pdf_invalido_e_informa_o_nome() {
        let pasta = PastaTemporaria::nova();
        let caminho = pasta.arquivo("quebrado.pdf");
        assert_eq!(
            contar_paginas(&caminho),
            Err(ErroDocumento {
                codigo: CodigoErro::PdfInvalido,
                nome: "quebrado.pdf".to_owned(),
            })
        );
        let lixo = gravar(&pasta, "lixo.pdf", &[0xFF; 4096]);
        assert_eq!(codigo_ao_contar(&lixo), Some(CodigoErro::PdfInvalido));
        let vazio = gravar(&pasta, "vazio.pdf", b"");
        assert_eq!(codigo_ao_contar(&vazio), Some(CodigoErro::PdfInvalido));
    }

    #[test]
    fn recusa_pdf_sem_paginas() {
        let pasta = PastaTemporaria::nova();
        let caminho = gravar(&pasta, "sem-paginas.pdf", &pdf(0, &[], ""));
        assert_eq!(codigo_ao_contar(&caminho), Some(CodigoErro::PdfInvalido));
    }

    #[test]
    fn recusa_pdf_criptografado() {
        let pasta = PastaTemporaria::nova();
        let hexa = "A".repeat(64);
        let cifra = format!("<< /Filter /Standard /V 1 /R 2 /O <{hexa}> /U <{hexa}> /P -4 >>");
        let caminho = gravar(
            &pasta,
            "cifrado.pdf",
            &pdf(2, &[&cifra], "/Encrypt 5 0 R /ID [<00> <00>]"),
        );
        assert_eq!(codigo_ao_contar(&caminho), Some(CodigoErro::Criptografado));
    }

    #[test]
    fn os_erros_do_caminho_valem_tambem_ao_contar() {
        let pasta = PastaTemporaria::nova();
        let sumiu = pasta.0.join("sumiu.pdf");
        assert_eq!(
            codigo_ao_contar(&sumiu.to_string_lossy()),
            Some(CodigoErro::NaoEncontrado)
        );
        let texto = pasta.arquivo("notas.txt");
        assert_eq!(codigo_ao_contar(&texto), Some(CodigoErro::NaoEPdf));
        assert_eq!(codigo_ao_contar(""), Some(CodigoErro::NaoEncontrado));
    }

    #[test]
    fn arquivo_acima_do_limite_nao_chega_a_ser_interpretado() {
        let pasta = PastaTemporaria::nova();
        let bytes = pdf(1, &[], "");
        let caminho = gravar(&pasta, "grande.pdf", &bytes);
        let limite = u64::try_from(bytes.len()).expect("tamanho do teste");
        assert_eq!(contar_com_limite(&caminho, limite), Ok(1));
        assert_eq!(
            contar_com_limite(&caminho, limite - 1).map_err(|erro| erro.codigo),
            Err(CodigoErro::Grande)
        );
    }

    #[test]
    fn leitura_devolve_exatamente_os_bytes_do_pdf_aceito() {
        let pasta = PastaTemporaria::nova();
        let bytes = pdf(3, &[], "");
        let caminho = gravar(&pasta, "tres.pdf", &bytes);
        assert_eq!(
            ler_com_limite_validado(&caminho, LIMITE_BYTES_PDF),
            Ok(bytes)
        );
    }

    #[test]
    fn leitura_recusa_arquivo_inexistente_com_o_nome() {
        let pasta = PastaTemporaria::nova();
        let sumiu = pasta.0.join("sumiu.pdf");
        assert_eq!(
            ler_validado(&sumiu.to_string_lossy()),
            Err(ErroDocumento {
                codigo: CodigoErro::NaoEncontrado,
                nome: "sumiu.pdf".to_owned(),
            })
        );
    }

    #[test]
    fn leitura_recusa_pdf_protegido_sem_entregar_bytes() {
        let pasta = PastaTemporaria::nova();
        let hexa = "A".repeat(64);
        let cifra = format!("<< /Filter /Standard /V 1 /R 2 /O <{hexa}> /U <{hexa}> /P -4 >>");
        let caminho = gravar(
            &pasta,
            "cifrado.pdf",
            &pdf(2, &[&cifra], "/Encrypt 5 0 R /ID [<00> <00>]"),
        );
        assert_eq!(
            ler_validado(&caminho).map_err(|erro| erro.codigo),
            Err(CodigoErro::Criptografado)
        );
    }

    #[test]
    fn leitura_recusa_o_que_a_barreira_do_pdf_nao_aceita() {
        let pasta = PastaTemporaria::nova();
        let quebrado = pasta.arquivo("quebrado.pdf");
        assert_eq!(
            ler_validado(&quebrado).map_err(|erro| erro.codigo),
            Err(CodigoErro::PdfInvalido)
        );
        let texto = pasta.arquivo("notas.txt");
        assert_eq!(
            ler_validado(&texto).map_err(|erro| erro.codigo),
            Err(CodigoErro::NaoEPdf)
        );
    }

    #[test]
    fn leitura_acima_do_limite_vira_recusa_antes_de_interpretar() {
        let pasta = PastaTemporaria::nova();
        let bytes = pdf(1, &[], "");
        let caminho = gravar(&pasta, "grande.pdf", &bytes);
        let limite = u64::try_from(bytes.len()).expect("tamanho do teste");
        assert_eq!(ler_com_limite_validado(&caminho, limite), Ok(bytes));
        assert_eq!(
            ler_com_limite_validado(&caminho, limite - 1).map_err(|erro| erro.codigo),
            Err(CodigoErro::Grande)
        );
    }

    #[test]
    fn aninhamento_profundo_nao_derruba_o_app() {
        let pasta = PastaTemporaria::nova();
        let fundo = format!("{}{}", "[".repeat(200_000), "]".repeat(200_000));
        let caminho = gravar(&pasta, "fundo.pdf", &pdf(1, &[&fundo], ""));
        // Basta nao derrubar o processo: o objeto nao e referenciado por pagina alguma.
        let _ = contar_paginas(&caminho);
    }

    #[test]
    fn falha_inesperada_identifica_o_arquivo() {
        assert_eq!(
            falha_inesperada("pasta/a.pdf"),
            ErroDocumento {
                codigo: CodigoErro::PdfInvalido,
                nome: "a.pdf".to_owned(),
            }
        );
    }
}
