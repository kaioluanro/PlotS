//! Comandos dos grupos salvos: uma selecao de impressoras e um modo, com um nome. Ficam no arquivo
//! de perfis, e so `plots-perfis` le e grava esse arquivo.

use crate::impressoras::{self, ListaImpressoras, GRAVACAO_DE_PERFIS};
use crate::trabalhos::ModoDoEnvio;
use plots_perfis::{ErroGrupo, ErroPerfis, Grupo, Perfis};
use serde::Serialize;
use std::path::{Path, PathBuf};
use std::sync::PoisonError;

/// A interface escolhe a frase pelo codigo. O detalhe do erro de leitura nao sai daqui: ele pode
/// trazer trechos do arquivo.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum FalhaDoGrupo {
    NomeVazio,
    NomeGrande,
    NomeInvalido,
    NomeRepetido,
    SemImpressoras,
    MuitasImpressoras,
    MuitosGrupos,
    Inexistente,
    /// O arquivo de perfis nao abriu; nada foi gravado por cima dele.
    Leitura,
    Gravacao,
}

impl From<ErroGrupo> for FalhaDoGrupo {
    fn from(erro: ErroGrupo) -> Self {
        match erro {
            ErroGrupo::NomeVazio => Self::NomeVazio,
            ErroGrupo::NomeGrande => Self::NomeGrande,
            ErroGrupo::NomeInvalido => Self::NomeInvalido,
            ErroGrupo::NomeRepetido => Self::NomeRepetido,
            ErroGrupo::SemImpressoras => Self::SemImpressoras,
            ErroGrupo::MuitasImpressoras => Self::MuitasImpressoras,
            ErroGrupo::MuitosGrupos => Self::MuitosGrupos,
            ErroGrupo::Inexistente => Self::Inexistente,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct GrupoSalvo {
    pub nome: String,
    pub impressoras: Vec<String>,
    pub modo: ModoDoEnvio,
}

/// O grupo pronto para aplicar: as impressoras que ainda existem e os nomes das que sumiram.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct GrupoAplicado {
    pub impressoras: Vec<String>,
    pub modo: ModoDoEnvio,
    pub ausentes: Vec<String>,
}

fn caminho_dos_perfis() -> Result<PathBuf, FalhaDoGrupo> {
    impressoras::caminho_dos_perfis_do_sistema().map_err(|_: ErroPerfis| FalhaDoGrupo::Leitura)
}

fn carregar(caminho: &Path) -> Result<Perfis, FalhaDoGrupo> {
    plots_perfis::carregar(caminho).map_err(|_| FalhaDoGrupo::Leitura)
}

fn salvos(perfis: &Perfis) -> Vec<GrupoSalvo> {
    perfis
        .grupos()
        .map(|(nome, grupo)| GrupoSalvo {
            nome: nome.to_owned(),
            impressoras: grupo.impressoras.clone(),
            modo: grupo.modo.into(),
        })
        .collect()
}

/// O que a lista mostra, e quantos grupos do arquivo ficaram de fora por estarem invalidos.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct ListaDeGrupos {
    pub grupos: Vec<GrupoSalvo>,
    pub ignorados: usize,
}

fn listar_em(caminho: &Path) -> Result<ListaDeGrupos, FalhaDoGrupo> {
    let perfis = carregar(caminho)?;
    Ok(ListaDeGrupos {
        grupos: salvos(&perfis),
        ignorados: perfis.grupos_ignorados(),
    })
}

/// Le, muda e grava de uma vez. Um arquivo que nao abriu nunca e substituido.
fn alterar_em(
    caminho: &Path,
    alterar: impl FnOnce(&mut Perfis) -> Result<(), ErroGrupo>,
) -> Result<Vec<GrupoSalvo>, FalhaDoGrupo> {
    let _vez = GRAVACAO_DE_PERFIS
        .lock()
        .unwrap_or_else(PoisonError::into_inner);
    let mut perfis = carregar(caminho)?;
    alterar(&mut perfis)?;
    plots_perfis::salvar(caminho, &perfis).map_err(|_| FalhaDoGrupo::Gravacao)?;
    Ok(salvos(&perfis))
}

/// O nome mostrado de uma impressora que sumiu: sem o prefixo do identificador.
fn nome_do_identificador(id: &str) -> String {
    id.strip_prefix("fila:").unwrap_or(id).to_owned()
}

fn aplicar(grupo: &Grupo, lista: &ListaImpressoras) -> GrupoAplicado {
    let (presentes, ausentes): (Vec<String>, Vec<String>) =
        grupo.impressoras.iter().cloned().partition(|id| {
            lista
                .impressoras
                .iter()
                .any(|impressora| impressora.id == *id)
        });
    GrupoAplicado {
        impressoras: presentes,
        modo: grupo.modo.into(),
        ausentes: ausentes
            .iter()
            .map(|id| nome_do_identificador(id))
            .collect(),
    }
}

fn aplicar_em(
    caminho: &Path,
    nome: &str,
    lista: &ListaImpressoras,
) -> Result<GrupoAplicado, FalhaDoGrupo> {
    let perfis = carregar(caminho)?;
    let grupo = perfis.grupo(nome).ok_or(FalhaDoGrupo::Inexistente)?;
    Ok(aplicar(grupo, lista))
}

/// O disco e o spooler podem demorar, por isso a thread propria em todos os comandos.
async fn em_thread<T: Send + 'static>(
    falha: FalhaDoGrupo,
    tarefa: impl FnOnce() -> Result<T, FalhaDoGrupo> + Send + 'static,
) -> Result<T, FalhaDoGrupo> {
    tauri::async_runtime::spawn_blocking(tarefa)
        .await
        .unwrap_or(Err(falha))
}

#[tauri::command]
pub async fn listar_grupos() -> Result<ListaDeGrupos, FalhaDoGrupo> {
    em_thread(FalhaDoGrupo::Leitura, || listar_em(&caminho_dos_perfis()?)).await
}

#[tauri::command]
pub async fn salvar_grupo(
    nome: String,
    impressoras: Vec<String>,
) -> Result<Vec<GrupoSalvo>, FalhaDoGrupo> {
    em_thread(FalhaDoGrupo::Gravacao, move || {
        alterar_em(&caminho_dos_perfis()?, |perfis| {
            // Todas as marcadas imprimem a mesma coisa. O campo fica no arquivo, que aceita os antigos.
            perfis.salvar_grupo(&nome, &impressoras, ModoDoEnvio::Espelho.into())
        })
    })
    .await
}

#[tauri::command]
pub async fn apagar_grupo(nome: String) -> Result<Vec<GrupoSalvo>, FalhaDoGrupo> {
    em_thread(FalhaDoGrupo::Gravacao, move || {
        alterar_em(&caminho_dos_perfis()?, |perfis| {
            perfis.apagar_grupo(&nome).map(|_| ())
        })
    })
    .await
}

#[tauri::command]
pub async fn aplicar_grupo(nome: String) -> Result<GrupoAplicado, FalhaDoGrupo> {
    em_thread(FalhaDoGrupo::Leitura, move || {
        aplicar_em(
            &caminho_dos_perfis()?,
            &nome,
            &impressoras::ler_do_sistema(),
        )
    })
    .await
}

#[cfg(test)]
mod testes {
    use super::*;
    use crate::impressoras::montar;
    use plots_core::distribuicao::ModoDistribuicao;
    use std::fs;
    use std::sync::atomic::{AtomicU64, Ordering};

    static SEQUENCIA: AtomicU64 = AtomicU64::new(0);

    struct Pasta(PathBuf);

    impl Pasta {
        fn nova() -> Self {
            let caminho = std::env::temp_dir().join(format!(
                "plots-grupos-teste-{}-{}",
                std::process::id(),
                SEQUENCIA.fetch_add(1, Ordering::Relaxed)
            ));
            fs::create_dir_all(&caminho).unwrap();
            Self(caminho)
        }

        fn arquivo(&self) -> PathBuf {
            self.0.join("perfis.toml")
        }
    }

    impl Drop for Pasta {
        fn drop(&mut self) {
            let _ = fs::remove_dir_all(&self.0);
        }
    }

    fn duas() -> Vec<String> {
        vec!["simulada:gaveta".to_owned(), "fila:Sumida".to_owned()]
    }

    #[test]
    #[cfg(feature = "simuladas")]
    fn salvar_listar_aplicar_e_apagar_pelo_mesmo_arquivo() {
        let pasta = Pasta::nova();
        let arquivo = pasta.arquivo();
        assert_eq!(
            listar_em(&arquivo),
            Ok(ListaDeGrupos {
                grupos: Vec::new(),
                ignorados: 0
            })
        );
        let depois = alterar_em(&arquivo, |perfis| {
            perfis.salvar_grupo("Escritório", &duas(), ModoDistribuicao::Divisao)
        })
        .unwrap();
        assert_eq!(depois[0].nome, "Escritório");
        assert_eq!(listar_em(&arquivo).unwrap().grupos, depois);

        let lista = montar(Ok(Vec::new()), Ok(Perfis::new()));
        let aplicado = aplicar_em(&arquivo, "Escritório", &lista).unwrap();
        assert_eq!(aplicado.impressoras, ["simulada:gaveta"]);
        assert_eq!(aplicado.modo, ModoDoEnvio::Divisao);
        assert_eq!(aplicado.ausentes, ["Sumida"]);

        let vazio = alterar_em(&arquivo, |perfis| {
            perfis.apagar_grupo("Escritório").map(|_| ())
        })
        .unwrap();
        assert!(vazio.is_empty());
        assert_eq!(
            aplicar_em(&arquivo, "Escritório", &lista),
            Err(FalhaDoGrupo::Inexistente)
        );
    }

    #[test]
    fn recusa_de_nome_chega_com_o_codigo_e_nao_muda_o_arquivo() {
        let pasta = Pasta::nova();
        let arquivo = pasta.arquivo();
        let salvar = |nome: &str| {
            alterar_em(&arquivo, |perfis| {
                perfis.salvar_grupo(nome, &duas(), ModoDistribuicao::Espelho)
            })
        };
        salvar("Sala").unwrap();
        let antes = fs::read(&arquivo).unwrap();
        assert_eq!(salvar(" sala "), Err(FalhaDoGrupo::NomeRepetido));
        assert_eq!(salvar("   "), Err(FalhaDoGrupo::NomeVazio));
        assert_eq!(salvar("a\u{202E}b"), Err(FalhaDoGrupo::NomeInvalido));
        assert_eq!(salvar(&"x".repeat(5_000)), Err(FalhaDoGrupo::NomeGrande));
        assert_eq!(fs::read(&arquivo).unwrap(), antes);
    }

    #[test]
    fn arquivo_corrompido_nao_e_substituido_por_nenhum_comando() {
        let pasta = Pasta::nova();
        let arquivo = pasta.arquivo();
        fs::write(&arquivo, "isto nao = = e toml").unwrap();
        let antes = fs::read(&arquivo).unwrap();
        let lista = montar(Ok(Vec::new()), Ok(Perfis::new()));
        assert_eq!(listar_em(&arquivo), Err(FalhaDoGrupo::Leitura));
        assert_eq!(
            aplicar_em(&arquivo, "Sala", &lista),
            Err(FalhaDoGrupo::Leitura)
        );
        assert_eq!(
            alterar_em(&arquivo, |perfis| {
                perfis.salvar_grupo("Sala", &duas(), ModoDistribuicao::Espelho)
            }),
            Err(FalhaDoGrupo::Leitura)
        );
        assert_eq!(
            alterar_em(&arquivo, |perfis| perfis.apagar_grupo("Sala").map(|_| ())),
            Err(FalhaDoGrupo::Leitura)
        );
        assert_eq!(fs::read(&arquivo).unwrap(), antes);
    }

    #[test]
    fn grupo_invalido_no_arquivo_vira_contagem_e_nao_impede_a_lista() {
        let pasta = Pasta::nova();
        let arquivo = pasta.arquivo();
        fs::write(
            &arquivo,
            "versao = 1\n\n[grupos.Certo]\nimpressoras = [\"fila:Sala\"]\nmodo = \"espelho\"\n\n[grupos.Errado]\nmodo = 3\n",
        )
        .unwrap();
        let lista = listar_em(&arquivo).unwrap();
        assert_eq!(lista.ignorados, 1);
        assert_eq!(lista.grupos.len(), 1);
    }

    #[test]
    fn gravar_modelo_e_grupo_ao_mesmo_tempo_mantem_as_duas_mudancas() {
        for rodada in 0..60 {
            let pasta = Pasta::nova();
            let arquivo = pasta.arquivo();
            plots_perfis::salvar(&arquivo, &Perfis::new()).unwrap();
            let barreira = std::sync::Barrier::new(2);
            std::thread::scope(|escopo| {
                escopo.spawn(|| {
                    barreira.wait();
                    impressoras::gravar_modelo(Ok(arquivo.clone()), "Sala", "laser").unwrap();
                });
                escopo.spawn(|| {
                    barreira.wait();
                    alterar_em(&arquivo, |perfis| {
                        perfis.salvar_grupo("Grupo", &duas(), ModoDistribuicao::Espelho)
                    })
                    .unwrap();
                });
            });
            let lidos = plots_perfis::carregar(&arquivo).unwrap();
            assert!(
                lidos.obter("Sala").is_some(),
                "modelo perdido na rodada {rodada}"
            );
            assert!(
                lidos.grupo("Grupo").is_some(),
                "grupo perdido na rodada {rodada}"
            );
        }
    }

    #[test]
    #[cfg(feature = "simuladas")]
    fn aplicar_ignora_quem_nao_existe_e_ainda_devolve_o_modo() {
        let grupo = Grupo {
            impressoras: vec![
                "fila:Fora".to_owned(),
                "simulada:traseira".to_owned(),
                "simulada:gaveta".to_owned(),
            ],
            modo: ModoDistribuicao::Espelho,
        };
        let aplicado = aplicar(&grupo, &montar(Ok(Vec::new()), Ok(Perfis::new())));
        assert_eq!(
            aplicado.impressoras,
            ["simulada:traseira", "simulada:gaveta"]
        );
        assert_eq!(aplicado.ausentes, ["Fora"]);
        assert_eq!(aplicado.modo, ModoDoEnvio::Espelho);
    }
}
