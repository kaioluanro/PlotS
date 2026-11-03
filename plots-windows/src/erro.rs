//! Falhas do envio ao spooler, com mensagem para quem está no terminal.

use std::path::PathBuf;

/// Comando que obtém a biblioteca de rasterização.
pub const COMANDO_BAIXAR_BIBLIOTECA: &str = "node scripts/baixar-pdfium.mjs";

/// Por que uma passada não pôde ser enviada ou acompanhada.
#[derive(Clone, Debug, PartialEq)]
pub enum ErroEnvio {
    /// O envio só existe no Windows.
    NaoSuportado,
    /// A biblioteca de rasterização não está em nenhum dos caminhos aceitos.
    BibliotecaAusente { procurados: Vec<PathBuf> },
    /// O caminho indicado para a biblioteca precisa ser absoluto.
    BibliotecaCaminhoRelativo(PathBuf),
    /// A biblioteca existe, mas não pôde ser carregada.
    BibliotecaInvalida(String),
    /// O PDF foi recusado ao ser aberto ou ao montar a passada.
    Pdf(String),
    /// A biblioteca falhou ao abrir ou desenhar a passada.
    Rasterizacao(String),
    /// A passada não tem página nenhuma.
    PassadaVazia,
    /// A passada cita uma página que o documento não tem.
    PaginaInexistente(usize),
    /// Dimensões da página fora do que dá para desenhar.
    TamanhoInvalido,
    /// A página, na resolução da fila, passa do limite de memória.
    MemoriaExcedida { largura: u64, altura: u64 },
    /// O papel é grande demais para ser informado ao driver.
    PapelGrandeDemais { largura_mm: f64, altura_mm: f64 },
    /// O driver não aplicou o papel pedido.
    PapelNaoAceito {
        pedido_mm: (f64, f64),
        obtido_mm: (f64, f64),
    },
    /// O driver não aceitou imprimir em uma face só.
    SimplexNaoAceito,
    /// O driver não aceitou imprimir nas duas faces, ou nem declara ter o recurso.
    DuasFacesNaoAceitas,
    /// O driver devolveu outro valor em um campo que o envio precisa fixar.
    ValorNaoAceito {
        campo: &'static str,
        pedido: i64,
        obtido: i64,
    },
    /// O driver não confirmou um valor que só faz sentido dizer em palavras.
    ValorNaoConfirmado { pedido: &'static str },
    /// A página em branco ficaria fora da área que a fila consegue imprimir.
    PaginaForaDaArea,
    /// A fila não aceita o desenho por bitmap que o envio usa.
    SemSuporteABitmap,
    /// O sistema recusou uma chamada; o código é o HRESULT.
    Sistema { chamada: &'static str, codigo: u32 },
    /// A resposta do sistema não tem a forma esperada.
    Resposta(&'static str),
    /// O nome de um arquivo ou fila não pode ser passado ao sistema.
    TextoInvalido,
    /// O arquivo de saída precisa ficar em uma pasta que existe.
    PastaDeSaidaInvalida(PathBuf),
    /// O arquivo de saída já existe e nunca é sobrescrito.
    ArquivoJaExiste(PathBuf),
    /// O trabalho terminou, mas o arquivo de saída não apareceu.
    ArquivoNaoGerado(PathBuf),
    /// Não há trabalho enviado para consultar.
    SemTrabalho,
}

impl std::fmt::Display for ErroEnvio {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::NaoSuportado => f.write_str("o envio à impressora só está disponível no Windows"),
            Self::BibliotecaAusente { procurados } => {
                let lista = procurados
                    .iter()
                    .map(|caminho| caminho.display().to_string())
                    .collect::<Vec<_>>()
                    .join(", ");
                write!(
                    f,
                    "não encontrei pdfium.dll (procurei em: {lista}). Rode '{COMANDO_BAIXAR_BIBLIOTECA}' e aponte PLOTS_PDFIUM_DIR para a pasta indicada por ele, ou coloque a DLL ao lado do executável"
                )
            }
            Self::BibliotecaCaminhoRelativo(caminho) => write!(
                f,
                "PLOTS_PDFIUM_DIR precisa ser um caminho absoluto; recebi {}",
                caminho.display()
            ),
            Self::BibliotecaInvalida(detalhe) => {
                write!(
                    f,
                    "não foi possível carregar pdfium.dll: {detalhe}. Rode '{COMANDO_BAIXAR_BIBLIOTECA}' para obter a versão certa"
                )
            }
            Self::Pdf(detalhe) => f.write_str(detalhe),
            Self::Rasterizacao(detalhe) => {
                write!(f, "não foi possível desenhar a página: {detalhe}")
            }
            Self::PassadaVazia => f.write_str("a passada não tem páginas"),
            Self::PaginaInexistente(numero) => write!(f, "a página {numero} não existe no PDF"),
            Self::TamanhoInvalido => {
                f.write_str("a página tem dimensões que não dão para desenhar")
            }
            Self::MemoriaExcedida { largura, altura } => write!(
                f,
                "a página daria {largura} × {altura} pixels na resolução da fila, acima do limite de {} MiB por página; escolha uma resolução menor no driver",
                crate::raster::LIMITE_BYTES_PAGINA / (1024 * 1024)
            ),
            Self::PapelGrandeDemais {
                largura_mm,
                altura_mm,
            } => write!(
                f,
                "o papel de {largura_mm:.0} × {altura_mm:.0} mm é grande demais para informar ao driver"
            ),
            Self::PapelNaoAceito {
                pedido_mm,
                obtido_mm,
            } => write!(
                f,
                "a fila não aplicou o papel de {:.0} × {:.0} mm e ficou em {:.0} × {:.0} mm; nada foi enviado",
                pedido_mm.0, pedido_mm.1, obtido_mm.0, obtido_mm.1
            ),
            Self::SimplexNaoAceito => {
                f.write_str("o driver não aceitou imprimir em uma face só; nada foi enviado")
            }
            Self::DuasFacesNaoAceitas => {
                f.write_str("o driver não aceitou imprimir nas duas faces; nada foi enviado")
            }
            Self::ValorNaoAceito {
                campo,
                pedido,
                obtido,
            } => write!(
                f,
                "o driver não aceitou {campo}: pedi {pedido} e ficou {obtido}; nada foi enviado"
            ),
            Self::ValorNaoConfirmado { pedido } => write!(
                f,
                "pedi {pedido}, e o driver não confirmou; nada foi enviado"
            ),
            Self::PaginaForaDaArea => f.write_str(
                "a página em branco ficaria fora da área que esta fila imprime; nada foi enviado",
            ),
            Self::SemSuporteABitmap => {
                f.write_str("esta fila não aceita páginas em bitmap; nada foi enviado")
            }
            Self::Sistema { chamada, codigo } => {
                write!(f, "o Windows recusou {chamada} (código {codigo:#010X})")
            }
            Self::Resposta(detalhe) => write!(f, "resposta inesperada do spooler: {detalhe}"),
            Self::TextoInvalido => {
                f.write_str("o nome da fila ou do arquivo tem caracteres que o Windows não aceita")
            }
            Self::PastaDeSaidaInvalida(pasta) => {
                write!(f, "a pasta de saída {} não existe", pasta.display())
            }
            Self::ArquivoJaExiste(caminho) => write!(
                f,
                "o arquivo {} já existe; escolha outra pasta ou remova o arquivo existente",
                caminho.display()
            ),
            Self::ArquivoNaoGerado(caminho) => write!(
                f,
                "o trabalho saiu da fila, mas o arquivo {} não apareceu",
                caminho.display()
            ),
            Self::SemTrabalho => f.write_str("não há trabalho enviado para acompanhar"),
        }
    }
}

impl std::error::Error for ErroEnvio {}

#[cfg(test)]
mod testes {
    use super::*;

    #[test]
    fn biblioteca_ausente_aponta_o_script_e_os_caminhos_procurados() {
        let texto = ErroEnvio::BibliotecaAusente {
            procurados: vec![PathBuf::from("C:/pasta/pdfium.dll")],
        }
        .to_string();
        assert!(texto.contains("scripts/baixar-pdfium.mjs"), "{texto}");
        assert!(texto.contains("C:/pasta/pdfium.dll"), "{texto}");
        assert!(texto.contains("PLOTS_PDFIUM_DIR"), "{texto}");
    }

    #[test]
    fn limite_de_memoria_explica_o_tamanho_e_o_que_fazer() {
        let texto = ErroEnvio::MemoriaExcedida {
            largura: 20_000,
            altura: 30_000,
        }
        .to_string();
        assert!(texto.contains("20000 × 30000"), "{texto}");
        assert!(texto.contains("MiB"), "{texto}");
    }
}
