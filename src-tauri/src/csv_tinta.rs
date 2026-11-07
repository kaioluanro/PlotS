//! Exportação da estimativa de tinta em CSV: uma linha por página da seleção e uma linha de
//! total.
//!
//! O texto inteiro é montado e validado antes de qualquer arquivo ser aberto, e a gravação troca
//! um temporário pelo destino só depois de escrever tudo. Um erro no meio nunca deixa a planilha
//! pela metade nem apaga um arquivo que já existia.

use plots_core::{DimensoesPagina, LIMITE_COPIAS};
use plots_info::{Cobertura, Consumo, Custo};
use std::collections::BTreeSet;
use std::ffi::{OsStr, OsString};
use std::fs::{self, OpenOptions};
use std::io::{ErrorKind, Write};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};

/// Cabeçalho da planilha, na ordem em que os valores aparecem.
pub const CABECALHO: &str = "numero;largura_mm;altura_mm;ciano_pct;magenta_pct;amarelo_pct;preto_pct;ciano_ml;magenta_ml;amarelo_ml;preto_ml;total_ml;custo";

/// Rótulo da primeira coluna da linha de total.
pub const ROTULO_TOTAL: &str = "total";

/// Maior quantidade de páginas aceita em uma exportação.
pub const LIMITE_PAGINAS: usize = 100_000;

/// Maior texto gerado, para uma seleção enorme não ocupar memória sem limite.
pub const LIMITE_TEXTO_BYTES: usize = 64 * 1024 * 1024;

const CASAS_DIMENSAO: usize = 3;
const CASAS_PERCENTUAL: usize = 4;
const CASAS_ML: usize = 6;
const CASAS_CUSTO: usize = 6;

static CONTADOR_TEMPORARIOS: AtomicU64 = AtomicU64::new(0);

/// Tinta e custo de uma página da seleção, para a planilha.
#[derive(Debug, Clone, PartialEq)]
pub struct PaginaDeTinta {
    pub numero: u32,
    pub dimensoes: DimensoesPagina,
    pub cobertura: Cobertura,
    pub tinta: Consumo,
    pub custo: Custo,
}

/// Páginas escolhidas e quantas cópias a estimativa considera no total.
#[derive(Debug, Clone, Copy)]
pub struct SelecaoDeTinta<'a> {
    pub paginas: &'a [PaginaDeTinta],
    pub copias: usize,
}

impl<'a> SelecaoDeTinta<'a> {
    pub fn nova(paginas: &'a [PaginaDeTinta], copias: usize) -> Self {
        Self { paginas, copias }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ErroCsv {
    CaminhoInvalido,
    PastaIndisponivel,
    CopiasInvalidas { copias: usize },
    SemPaginas,
    PaginasDemais { paginas: usize, limite: usize },
    NumeroInvalido { pagina: u32 },
    NumeroRepetido { pagina: u32 },
    DimensaoInvalida { pagina: u32 },
    CoberturaInvalida { pagina: u32 },
    ConsumoInvalido { pagina: u32 },
    CustoInvalido { pagina: u32 },
    TotalInvalido { canal: usize },
    TextoGrande { bytes: usize, limite: usize },
    NaoGravou { detalhe: String },
}

impl std::fmt::Display for ErroCsv {
    fn fmt(&self, saida: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::CaminhoInvalido => write!(saida, "o caminho da planilha não é um arquivo"),
            Self::PastaIndisponivel => {
                write!(saida, "a pasta de destino da planilha não existe")
            }
            Self::CopiasInvalidas { copias } => write!(
                saida,
                "o número de cópias deve estar entre 1 e {LIMITE_COPIAS}, não {copias}"
            ),
            Self::SemPaginas => write!(saida, "não há páginas para exportar"),
            Self::PaginasDemais { paginas, limite } => write!(
                saida,
                "a exportação pediu {paginas} páginas e o limite é {limite}"
            ),
            Self::NumeroInvalido { pagina } => {
                write!(saida, "a página {pagina} não é um número válido")
            }
            Self::NumeroRepetido { pagina } => {
                write!(saida, "a página {pagina} aparece mais de uma vez")
            }
            Self::DimensaoInvalida { pagina } => write!(
                saida,
                "as dimensões da página {pagina} precisam ser positivas e finitas"
            ),
            Self::CoberturaInvalida { pagina } => write!(
                saida,
                "a cobertura da página {pagina} precisa estar entre 0 e 100"
            ),
            Self::ConsumoInvalido { pagina } => write!(
                saida,
                "o consumo da página {pagina} precisa ser finito e não negativo"
            ),
            Self::CustoInvalido { pagina } => write!(
                saida,
                "o custo da página {pagina} precisa ser finito e não negativo"
            ),
            Self::TotalInvalido { canal } => {
                write!(saida, "o total do canal {canal} não é um número finito")
            }
            Self::TextoGrande { bytes, limite } => write!(
                saida,
                "a planilha tem {bytes} bytes e passa do limite de {limite}"
            ),
            Self::NaoGravou { detalhe } => {
                write!(saida, "não foi possível gravar a planilha: {detalhe}")
            }
        }
    }
}

impl std::error::Error for ErroCsv {}

/// Monta o texto da planilha. As linhas de página mostram uma cópia de cada página; a linha de
/// total mostra a seleção inteira, já multiplicada pelas cópias.
pub fn texto_csv(selecao: &SelecaoDeTinta<'_>) -> Result<String, ErroCsv> {
    validar(selecao)?;
    let mut texto = String::with_capacity(
        selecao
            .paginas
            .len()
            .saturating_mul(128)
            .saturating_add(256),
    );
    texto.push_str(CABECALHO);
    texto.push('\n');

    let mut soma = [0.0f64; 4];
    let mut soma_custo = 0.0f64;
    for pagina in selecao.paginas {
        let campos = campos_da_pagina(pagina);
        texto.push_str(&campos.join(";"));
        texto.push('\n');
        for (acumulado, valor) in soma.iter_mut().zip(pagina.tinta.valores()) {
            *acumulado += valor;
        }
        soma_custo += pagina.custo.total();
        if texto.len() > LIMITE_TEXTO_BYTES {
            return Err(ErroCsv::TextoGrande {
                bytes: texto.len(),
                limite: LIMITE_TEXTO_BYTES,
            });
        }
    }

    let copias = selecao.copias as f64;
    let mut campos_total = Vec::with_capacity(13);
    campos_total.push(ROTULO_TOTAL.to_owned());
    for _ in 0..6 {
        campos_total.push(String::new());
    }
    let mut total_dos_canais = 0.0f64;
    for (canal, valor) in soma.iter().enumerate() {
        let total = valor * copias;
        if !total.is_finite() || total < 0.0 {
            return Err(ErroCsv::TotalInvalido { canal });
        }
        total_dos_canais += total;
        campos_total.push(decimais(total, CASAS_ML));
    }
    let custo_total = soma_custo * copias;
    if !total_dos_canais.is_finite() || !custo_total.is_finite() || custo_total < 0.0 {
        return Err(ErroCsv::TotalInvalido { canal: 0 });
    }
    campos_total.push(decimais(total_dos_canais, CASAS_ML));
    campos_total.push(decimais(custo_total, CASAS_CUSTO));
    texto.push_str(&campos_total.join(";"));
    texto.push('\n');

    if texto.len() > LIMITE_TEXTO_BYTES {
        return Err(ErroCsv::TextoGrande {
            bytes: texto.len(),
            limite: LIMITE_TEXTO_BYTES,
        });
    }
    Ok(texto)
}

/// Grava a planilha no caminho indicado. O texto é validado antes de abrir o destino; a troca do
/// temporário pelo arquivo final só acontece depois de tudo escrito e sincronizado.
pub fn gravar_csv(caminho: &Path, selecao: &SelecaoDeTinta<'_>) -> Result<(), ErroCsv> {
    let texto = texto_csv(selecao)?;
    let Some(nome) = caminho.file_name() else {
        return Err(ErroCsv::CaminhoInvalido);
    };
    if caminho.as_os_str().is_empty() || caminho.is_dir() {
        return Err(ErroCsv::CaminhoInvalido);
    }
    let pasta = match caminho.parent() {
        Some(pasta) if !pasta.as_os_str().is_empty() => pasta,
        _ => Path::new("."),
    };
    if !pasta.is_dir() {
        return Err(ErroCsv::PastaIndisponivel);
    }
    let (mut arquivo, temporario) = criar_temporario(pasta, nome)?;
    let escrito = arquivo.write_all(texto.as_bytes()).and_then(|()| {
        arquivo.sync_all()?;
        Ok(())
    });
    drop(arquivo);
    let resultado = match escrito {
        Ok(()) => fs::rename(&temporario, caminho),
        Err(falha) => Err(falha),
    };
    if resultado.is_err() {
        let _ = fs::remove_file(&temporario);
    }
    resultado.map_err(|falha| ErroCsv::NaoGravou {
        detalhe: falha.to_string(),
    })
}

fn criar_temporario(pasta: &Path, nome: &OsStr) -> Result<(fs::File, PathBuf), ErroCsv> {
    for _ in 0..8 {
        let temporario = caminho_temporario(pasta, nome);
        match OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&temporario)
        {
            Ok(arquivo) => return Ok((arquivo, temporario)),
            Err(falha) if falha.kind() == ErrorKind::AlreadyExists => continue,
            Err(falha) => {
                return Err(ErroCsv::NaoGravou {
                    detalhe: falha.to_string(),
                })
            }
        }
    }
    Err(ErroCsv::NaoGravou {
        detalhe: "não há nome livre para o arquivo temporário".to_owned(),
    })
}

fn caminho_temporario(pasta: &Path, nome: &OsStr) -> PathBuf {
    let sequencia = CONTADOR_TEMPORARIOS.fetch_add(1, Ordering::Relaxed);
    let mut temporario = OsString::from(".");
    temporario.push(nome);
    temporario.push(format!(".{}-{sequencia}.tmp", std::process::id()));
    pasta.join(temporario)
}

fn validar(selecao: &SelecaoDeTinta<'_>) -> Result<(), ErroCsv> {
    if !(1..=LIMITE_COPIAS).contains(&selecao.copias) {
        return Err(ErroCsv::CopiasInvalidas {
            copias: selecao.copias,
        });
    }
    if selecao.paginas.is_empty() {
        return Err(ErroCsv::SemPaginas);
    }
    if selecao.paginas.len() > LIMITE_PAGINAS {
        return Err(ErroCsv::PaginasDemais {
            paginas: selecao.paginas.len(),
            limite: LIMITE_PAGINAS,
        });
    }
    let mut vistas = BTreeSet::new();
    for pagina in selecao.paginas {
        validar_pagina(pagina)?;
        if !vistas.insert(pagina.numero) {
            return Err(ErroCsv::NumeroRepetido {
                pagina: pagina.numero,
            });
        }
    }
    Ok(())
}

fn validar_pagina(pagina: &PaginaDeTinta) -> Result<(), ErroCsv> {
    if pagina.numero == 0 {
        return Err(ErroCsv::NumeroInvalido {
            pagina: pagina.numero,
        });
    }
    let largura = pagina.dimensoes.largura_mm;
    let altura = pagina.dimensoes.altura_mm;
    if !largura.is_finite() || !altura.is_finite() || largura <= 0.0 || altura <= 0.0 {
        return Err(ErroCsv::DimensaoInvalida {
            pagina: pagina.numero,
        });
    }
    for valor in pagina.cobertura.valores() {
        if !valor.is_finite() || !(0.0..=100.0).contains(&valor) {
            return Err(ErroCsv::CoberturaInvalida {
                pagina: pagina.numero,
            });
        }
    }
    for valor in pagina.tinta.valores() {
        if !valor.is_finite() || valor < 0.0 {
            return Err(ErroCsv::ConsumoInvalido {
                pagina: pagina.numero,
            });
        }
    }
    for valor in pagina.custo.valores() {
        if !valor.is_finite() || valor < 0.0 {
            return Err(ErroCsv::CustoInvalido {
                pagina: pagina.numero,
            });
        }
    }
    Ok(())
}

fn campos_da_pagina(pagina: &PaginaDeTinta) -> Vec<String> {
    let mut campos = Vec::with_capacity(13);
    campos.push(pagina.numero.to_string());
    campos.push(decimais(pagina.dimensoes.largura_mm, CASAS_DIMENSAO));
    campos.push(decimais(pagina.dimensoes.altura_mm, CASAS_DIMENSAO));
    for valor in pagina.cobertura.valores() {
        campos.push(decimais(valor, CASAS_PERCENTUAL));
    }
    for valor in pagina.tinta.valores() {
        campos.push(decimais(valor, CASAS_ML));
    }
    campos.push(decimais(pagina.tinta.total(), CASAS_ML));
    campos.push(decimais(pagina.custo.total(), CASAS_CUSTO));
    campos
}

/// Decimal com ponto, sem `-0` e sem depender do idioma do sistema.
fn decimais(valor: f64, casas: usize) -> String {
    let valor = if valor == 0.0 { 0.0 } else { valor };
    format!("{:.*}", casas, valor)
}

#[cfg(test)]
mod testes {
    use super::*;
    use std::fs;
    use std::path::PathBuf;
    use std::sync::atomic::AtomicU64;

    static CONTADOR: AtomicU64 = AtomicU64::new(0);

    struct Pasta(PathBuf);

    impl Pasta {
        fn nova() -> Self {
            let sequencia = CONTADOR.fetch_add(1, Ordering::Relaxed);
            let caminho = std::env::temp_dir().join(format!(
                "plots-csv-teste-{}-{sequencia}",
                std::process::id()
            ));
            fs::create_dir_all(&caminho).expect("criar pasta do teste");
            Self(caminho)
        }

        fn caminho(&self, nome: &str) -> PathBuf {
            self.0.join(nome)
        }

        fn temporarios(&self) -> Vec<String> {
            fs::read_dir(&self.0)
                .expect("ler a pasta do teste")
                .flatten()
                .map(|entrada| entrada.file_name().to_string_lossy().into_owned())
                .filter(|nome| nome.ends_with(".tmp"))
                .collect()
        }
    }

    impl Drop for Pasta {
        fn drop(&mut self) {
            let _ = fs::remove_dir_all(&self.0);
        }
    }

    fn pagina(numero: u32) -> PaginaDeTinta {
        PaginaDeTinta {
            numero,
            dimensoes: DimensoesPagina::new(210.0, 297.0),
            cobertura: Cobertura::new(10.0, 20.0, 30.0, 40.0),
            tinta: Consumo::new(0.1, 0.2, 0.3, 0.4),
            custo: Custo::new(0.01, 0.02, 0.03, 0.04),
        }
    }

    #[test]
    fn cabecalho_tem_uma_coluna_por_valor() {
        assert_eq!(
            CABECALHO,
            "numero;largura_mm;altura_mm;ciano_pct;magenta_pct;amarelo_pct;preto_pct;\
             ciano_ml;magenta_ml;amarelo_ml;preto_ml;total_ml;custo"
        );
    }

    #[test]
    fn selecao_de_quatro_paginas_tem_uma_linha_por_pagina_e_o_total_com_copias() {
        let paginas = vec![pagina(2), pagina(5), pagina(7), pagina(9)];
        let selecao = SelecaoDeTinta::nova(&paginas, 2);
        let texto = texto_csv(&selecao).expect("a seleção deveria virar CSV");
        let linhas: Vec<&str> = texto.lines().collect();
        assert_eq!(linhas.len(), 1 + 4 + 1, "cabeçalho, quatro páginas e total");
        assert_eq!(linhas[0], CABECALHO);
        assert_eq!(
            linhas[1],
            "2;210.000;297.000;10.0000;20.0000;30.0000;40.0000;\
             0.100000;0.200000;0.300000;0.400000;1.000000;0.100000"
        );
        assert_eq!(
            linhas[4],
            "9;210.000;297.000;10.0000;20.0000;30.0000;40.0000;\
             0.100000;0.200000;0.300000;0.400000;1.000000;0.100000"
        );
        // Quatro páginas, uma cópia cada, repetidas por duas cópias no total.
        assert_eq!(
            linhas[5],
            "total;;;;;;;0.800000;1.600000;2.400000;3.200000;8.000000;0.800000"
        );
        assert!(texto.contains("1.000000"), "decimal com ponto");
        assert!(!texto.contains(','), "nenhuma vírgula decimal");
    }

    #[test]
    fn uma_copia_faz_o_total_bater_com_as_linhas() {
        let paginas = vec![pagina(4)];
        let texto = texto_csv(&SelecaoDeTinta::nova(&paginas, 1)).expect("CSV");
        let linhas: Vec<&str> = texto.lines().collect();
        assert_eq!(
            linhas[2],
            "total;;;;;;;0.100000;0.200000;0.300000;0.400000;1.000000;0.100000"
        );
    }

    #[test]
    fn valores_hostis_sao_recusados_sem_gerar_texto() {
        let mut ruim = pagina(1);
        ruim.cobertura.c = f64::NAN;
        assert_eq!(
            texto_csv(&SelecaoDeTinta::nova(&[ruim], 1)),
            Err(ErroCsv::CoberturaInvalida { pagina: 1 })
        );

        let mut ruim = pagina(1);
        ruim.dimensoes.largura_mm = 0.0;
        assert_eq!(
            texto_csv(&SelecaoDeTinta::nova(&[ruim], 1)),
            Err(ErroCsv::DimensaoInvalida { pagina: 1 })
        );

        let mut ruim = pagina(1);
        ruim.tinta.k = f64::INFINITY;
        assert_eq!(
            texto_csv(&SelecaoDeTinta::nova(&[ruim], 1)),
            Err(ErroCsv::ConsumoInvalido { pagina: 1 })
        );

        let mut zero = pagina(0);
        zero.numero = 0;
        assert_eq!(
            texto_csv(&SelecaoDeTinta::nova(&[zero], 1)),
            Err(ErroCsv::NumeroInvalido { pagina: 0 })
        );

        let paginas = vec![pagina(1), pagina(1)];
        assert_eq!(
            texto_csv(&SelecaoDeTinta::nova(&paginas, 1)),
            Err(ErroCsv::NumeroRepetido { pagina: 1 })
        );

        let paginas = vec![pagina(1)];
        assert_eq!(
            texto_csv(&SelecaoDeTinta::nova(&paginas, 0)),
            Err(ErroCsv::CopiasInvalidas { copias: 0 })
        );
        assert_eq!(
            texto_csv(&SelecaoDeTinta::nova(&[], 1)),
            Err(ErroCsv::SemPaginas)
        );
    }

    #[test]
    fn gravacao_troca_o_arquivo_e_nao_deixa_temporario() {
        let pasta = Pasta::nova();
        let destino = pasta.caminho("tinta.csv");
        fs::write(&destino, "conteúdo antigo").expect("gravar sentinela");

        let paginas = vec![pagina(2), pagina(5)];
        gravar_csv(&destino, &SelecaoDeTinta::nova(&paginas, 1)).expect("gravar CSV");
        let texto = fs::read_to_string(&destino).expect("ler CSV");
        assert!(texto.starts_with(CABECALHO));
        assert!(texto.ends_with('\n'));
        assert_eq!(texto.lines().count(), 1 + 2 + 1);
        assert!(pasta.temporarios().is_empty());
    }

    #[test]
    fn erro_de_validacao_nao_toca_no_arquivo_existente() {
        let pasta = Pasta::nova();
        let destino = pasta.caminho("tinta.csv");
        fs::write(&destino, "sentinela").expect("gravar sentinela");

        let mut ruim = pagina(1);
        ruim.custo.c = f64::NAN;
        let erro = gravar_csv(&destino, &SelecaoDeTinta::nova(&[ruim], 1)).unwrap_err();
        assert_eq!(erro, ErroCsv::CustoInvalido { pagina: 1 });
        assert_eq!(fs::read_to_string(&destino).unwrap(), "sentinela");
        assert!(pasta.temporarios().is_empty());
    }

    #[test]
    fn caminho_invalido_nao_apaga_nada() {
        let pasta = Pasta::nova();
        let subpasta = pasta.caminho("ja-existe");
        fs::create_dir(&subpasta).expect("criar pasta de destino");
        let dentro = subpasta.join("importante.txt");
        fs::write(&dentro, "não apagar").expect("gravar arquivo dentro da pasta");

        let paginas = vec![pagina(1)];
        let selecao = SelecaoDeTinta::nova(&paginas, 1);
        assert_eq!(
            gravar_csv(&subpasta, &selecao),
            Err(ErroCsv::CaminhoInvalido)
        );
        assert_eq!(
            gravar_csv(Path::new(""), &selecao),
            Err(ErroCsv::CaminhoInvalido)
        );
        assert_eq!(fs::read_to_string(&dentro).unwrap(), "não apagar");

        let sem_pasta = pasta.caminho("nao-existe").join("tinta.csv");
        assert_eq!(
            gravar_csv(&sem_pasta, &selecao),
            Err(ErroCsv::PastaIndisponivel)
        );
    }
}
