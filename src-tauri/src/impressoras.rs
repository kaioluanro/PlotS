//! Lista de impressoras da barra lateral: filas do sistema, modo de cada uma e impressoras simuladas.
//!
//! Nenhuma pagina e enviada e nenhuma fila e alterada. O arquivo de perfis so muda quando o
//! usuario escolhe um modelo, com a gravacao atomica de `plots-perfis`.

use plots_core::ModeloImpressora;
use plots_perfis::{EntradaPerfil, ErroPerfis, Perfis};
use plots_windows::{Condicao, ErroFilas, Fila};
use serde::Serialize;
use std::collections::HashSet;
use std::ffi::OsString;
use std::path::PathBuf;
use std::sync::{Mutex, PoisonError};

/// Variavel que aponta outro arquivo de perfis no lugar do arquivo do usuario.
pub const VARIAVEL_PERFIS: &str = "PLOTS_PERFIS";

/// Nomes iguais aos que a linha de comando reconhece para as impressoras simuladas.
const NOME_SIMULADA_TRASEIRA: &str = "Simulada — Traseira";
const NOME_SIMULADA_GAVETA: &str = "Simulada — Gaveta";

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum Tipo {
    Fila,
    Simulada,
}

/// O que se sabe sobre a configuracao da impressora.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum Modo {
    /// Perfil pronto do jato de tinta.
    Jato,
    /// Perfil pronto da laser.
    Laser,
    /// Perfil salvo sem modelo: os valores nao dizem de que tipo a maquina e.
    Personalizado,
    /// Os perfis foram lidos e nenhum tem o nome exato desta fila.
    SemPerfil,
    /// Os perfis nao puderam ser lidos: nao se sabe se a fila tem perfil.
    Desconhecido,
}

/// Gravidade do estado, usada so para a cor do ponto; o texto vem sempre junto.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum Nivel {
    Pronta,
    Ativa,
    Atencao,
    Problema,
}

/// Uma condicao da fila em forma de codigo; a interface traduz.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct CondicaoLida {
    pub codigo: &'static str,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub detalhe: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct Impressora {
    /// Chave da selecao. Uma fila real chamada como uma simulada continua sendo outra impressora.
    pub id: String,
    pub nome: String,
    pub tipo: Tipo,
    pub nivel: Nivel,
    pub condicoes: Vec<CondicaoLida>,
    pub modo: Modo,
}

/// Motivo de um aviso na lista. A interface traduz o codigo.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum CodigoAviso {
    Filas,
    PerfisPasta,
    PerfisLeitura,
    PerfisGrande,
    PerfisInvalido,
    PerfisVersao,
    ModeloNaoGravado,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct Aviso {
    pub codigo: CodigoAviso,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub detalhe: Option<String>,
    /// Arquivo de perfis envolvido na falha, para a recuperacao dizer onde agir.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub caminho: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct ListaImpressoras {
    pub impressoras: Vec<Impressora>,
    pub avisos: Vec<Aviso>,
}

/// Uma gravacao por vez: duas escolhas seguidas nao podem ler o mesmo arquivo e perder uma delas.
static GRAVACAO_DE_MODELO: Mutex<()> = Mutex::new(());

/// Grava o perfil pronto do modelo no arquivo de perfis do sistema. Se falhar, nada muda.
pub fn definir_modelo(fila: &str, modelo: &str) -> Result<(), Aviso> {
    let caminho = caminho_dos_perfis(std::env::var_os(VARIAVEL_PERFIS));
    gravar_modelo(caminho, fila, modelo)
}

fn gravar_modelo(
    caminho: Result<PathBuf, ErroPerfis>,
    fila: &str,
    codigo: &str,
) -> Result<(), Aviso> {
    let (Some(modelo), false) = (ModeloImpressora::de_codigo(codigo), fila.trim().is_empty())
    else {
        return Err(falha_ao_gravar(fila));
    };
    let _vez = GRAVACAO_DE_MODELO
        .lock()
        .unwrap_or_else(PoisonError::into_inner);
    let aviso = |erro: ErroPerfis| Aviso {
        caminho: caminho_de_perfis(&erro),
        ..falha_ao_gravar(fila)
    };
    let caminho = caminho.map_err(aviso)?;
    let mut perfis = plots_perfis::carregar(&caminho).map_err(aviso)?;
    perfis.definir_modelo(fila, modelo);
    plots_perfis::salvar(&caminho, &perfis).map_err(aviso)
}

/// Aviso de que o modelo desta fila nao foi gravado.
pub fn falha_ao_gravar(fila: &str) -> Aviso {
    Aviso {
        codigo: CodigoAviso::ModeloNaoGravado,
        detalhe: Some(fila.to_owned()),
        caminho: None,
    }
}

/// Le as filas e os perfis do sistema. Uma falha em qualquer um vira aviso.
pub fn ler_do_sistema() -> ListaImpressoras {
    let caminho = caminho_dos_perfis(std::env::var_os(VARIAVEL_PERFIS));
    let perfis = caminho.and_then(|caminho| plots_perfis::carregar(&caminho));
    montar(plots_windows::listar_filas(), perfis)
}

/// Resposta quando a propria leitura foi interrompida: ao menos as simuladas continuam na lista.
pub fn sem_leitura() -> ListaImpressoras {
    montar(
        Err(ErroFilas::Resposta("a leitura foi interrompida")),
        Ok(Perfis::new()),
    )
}

/// Valor vazio conta como ausente, para nao tentar abrir um caminho sem nome.
fn caminho_dos_perfis(variavel: Option<OsString>) -> Result<PathBuf, ErroPerfis> {
    match variavel {
        Some(valor) if !valor.is_empty() => Ok(PathBuf::from(valor)),
        _ => plots_perfis::caminho_padrao(),
    }
}

/// Junta o que foi lido. Fora do Windows nao ha filas a ler, e isso nao e uma falha.
pub fn montar(
    filas: Result<Vec<Fila>, ErroFilas>,
    perfis: Result<Perfis, ErroPerfis>,
) -> ListaImpressoras {
    let mut avisos = Vec::new();
    let filas = filas.unwrap_or_else(|erro| {
        if let Some(aviso) = aviso_de_filas(&erro) {
            avisos.push(aviso);
        }
        Vec::new()
    });
    let perfis = match perfis {
        Ok(perfis) => Some(perfis),
        Err(erro) => {
            avisos.push(Aviso {
                codigo: codigo_de_perfis(&erro),
                detalhe: None,
                caminho: caminho_de_perfis(&erro),
            });
            None
        }
    };

    let mut vistas = HashSet::new();
    let mut impressoras: Vec<Impressora> = filas
        .iter()
        .filter(|fila| vistas.insert(fila.nome.as_str()))
        .map(|fila| impressora_da_fila(fila, perfis.as_ref()))
        .collect();
    impressoras.push(simulada(
        "simulada:traseira",
        NOME_SIMULADA_TRASEIRA,
        Modo::Jato,
    ));
    impressoras.push(simulada(
        "simulada:gaveta",
        NOME_SIMULADA_GAVETA,
        Modo::Laser,
    ));
    ListaImpressoras {
        impressoras,
        avisos,
    }
}

fn simulada(id: &str, nome: &str, modo: Modo) -> Impressora {
    Impressora {
        id: id.to_owned(),
        nome: nome.to_owned(),
        tipo: Tipo::Simulada,
        nivel: Nivel::Pronta,
        condicoes: Vec::new(),
        modo,
    }
}

fn impressora_da_fila(fila: &Fila, perfis: Option<&Perfis>) -> Impressora {
    let modo = match perfis {
        None => Modo::Desconhecido,
        Some(perfis) => match perfis.obter(&fila.nome) {
            None => Modo::SemPerfil,
            Some(entrada) => modo_da_entrada(entrada),
        },
    };
    let mut nivel = Nivel::Pronta;
    let mut condicoes = Vec::new();
    for condicao in fila.estado.condicoes() {
        let (lida, gravidade) = ler_condicao(condicao);
        condicoes.push(lida);
        nivel = nivel.max(gravidade);
    }
    Impressora {
        id: format!("fila:{}", fila.nome),
        nome: fila.nome.clone(),
        tipo: Tipo::Fila,
        nivel,
        condicoes,
        modo,
    }
}

/// O modo vem so do campo `modelo`: os valores do perfil nao dizem de que tipo a maquina e.
fn modo_da_entrada(entrada: &EntradaPerfil) -> Modo {
    match entrada.modelo {
        Some(ModeloImpressora::JatoDeTinta) => Modo::Jato,
        Some(ModeloImpressora::Laser) => Modo::Laser,
        None => Modo::Personalizado,
    }
}

fn aviso_de_filas(erro: &ErroFilas) -> Option<Aviso> {
    let detalhe = match erro {
        ErroFilas::NaoSuportado => return None,
        ErroFilas::Sistema(codigo) => Some(format!("{codigo:#010X}")),
        ErroFilas::Grande { .. } | ErroFilas::Resposta(_) => None,
    };
    Some(Aviso {
        codigo: CodigoAviso::Filas,
        detalhe,
        caminho: None,
    })
}

fn caminho_de_perfis(erro: &ErroPerfis) -> Option<String> {
    match erro {
        ErroPerfis::Leitura { caminho, .. }
        | ErroPerfis::Gravacao { caminho, .. }
        | ErroPerfis::ArquivoGrande { caminho }
        | ErroPerfis::TomlInvalido { caminho, .. }
        | ErroPerfis::VersaoDesconhecida { caminho, .. } => Some(caminho.display().to_string()),
        ErroPerfis::PastaUsuarioIndisponivel
        | ErroPerfis::Serializacao { .. }
        | ErroPerfis::ConteudoGrande => None,
    }
}

fn codigo_de_perfis(erro: &ErroPerfis) -> CodigoAviso {
    match erro {
        ErroPerfis::PastaUsuarioIndisponivel => CodigoAviso::PerfisPasta,
        ErroPerfis::ArquivoGrande { .. } | ErroPerfis::ConteudoGrande => CodigoAviso::PerfisGrande,
        ErroPerfis::TomlInvalido { .. } => CodigoAviso::PerfisInvalido,
        ErroPerfis::VersaoDesconhecida { .. } => CodigoAviso::PerfisVersao,
        ErroPerfis::Leitura { .. }
        | ErroPerfis::Gravacao { .. }
        | ErroPerfis::Serializacao { .. } => CodigoAviso::PerfisLeitura,
    }
}

/// Sem braco generico: uma condicao nova nao compila ate receber codigo e gravidade.
fn ler_condicao(condicao: &Condicao) -> (CondicaoLida, Nivel) {
    use Nivel::{Atencao, Ativa, Problema};
    let (codigo, nivel) = match condicao {
        Condicao::Pausada => ("pausada", Atencao),
        Condicao::Offline => ("offline", Problema),
        Condicao::SemPapel => ("sem_papel", Problema),
        Condicao::Atolada => ("atolada", Problema),
        Condicao::Erro => ("erro", Problema),
        Condicao::ExclusaoPendente => ("exclusao_pendente", Atencao),
        Condicao::AlimentacaoManual => ("alimentacao_manual", Atencao),
        Condicao::ProblemaComPapel => ("problema_com_papel", Problema),
        Condicao::RecebendoDados => ("recebendo_dados", Ativa),
        Condicao::Ocupada => ("ocupada", Ativa),
        Condicao::Imprimindo => ("imprimindo", Ativa),
        Condicao::BandejaCheia => ("bandeja_cheia", Atencao),
        Condicao::Indisponivel => ("indisponivel", Problema),
        Condicao::EmEspera => ("em_espera", Atencao),
        Condicao::Processando => ("processando", Ativa),
        Condicao::Inicializando => ("inicializando", Ativa),
        Condicao::Aquecendo => ("aquecendo", Ativa),
        Condicao::PoucoToner => ("pouco_toner", Atencao),
        Condicao::SemToner => ("sem_toner", Problema),
        Condicao::PaginaNaoImpressa => ("pagina_nao_impressa", Problema),
        Condicao::IntervencaoDoUsuario => ("intervencao_do_usuario", Problema),
        Condicao::SemMemoria => ("sem_memoria", Problema),
        Condicao::TampaAberta => ("tampa_aberta", Problema),
        Condicao::ServidorDesconhecido => ("servidor_desconhecido", Problema),
        Condicao::EconomiaDeEnergia => ("economia_de_energia", Ativa),
        Condicao::ServidorOffline => ("servidor_offline", Problema),
        Condicao::DriverDesatualizado => ("driver_desatualizado", Atencao),
        Condicao::Outro(bit) => {
            return (
                CondicaoLida {
                    codigo: "outro",
                    detalhe: Some(format!("{bit:#010X}")),
                },
                Atencao,
            )
        }
    };
    (
        CondicaoLida {
            codigo,
            detalhe: None,
        },
        nivel,
    )
}

#[cfg(test)]
mod testes {
    use super::*;
    use plots_core::PerfilImpressora;
    use plots_windows::Estado;
    use std::path::Path;

    const OFFLINE: u32 = 0x0000_0080;
    const IMPRIMINDO: u32 = 0x0000_0400;
    const POUCO_TONER: u32 = 0x0002_0000;

    fn fila(nome: &str, bits: u32) -> Fila {
        Fila {
            nome: nome.to_owned(),
            driver: "Driver".to_owned(),
            porta: "USB001".to_owned(),
            padrao: false,
            estado: Estado::de_bits(bits),
        }
    }

    fn perfis_de(entradas: &[(&str, bool)]) -> Perfis {
        let mut perfis = Perfis::new();
        for (nome, face_para_cima) in entradas {
            perfis.inserir(
                nome,
                EntradaPerfil {
                    perfil: PerfilImpressora {
                        passada1_inversa: *face_para_cima,
                        ..PerfilImpressora::default()
                    },
                    ..EntradaPerfil::default()
                },
            );
        }
        perfis
    }

    fn falha_de_toml() -> ErroPerfis {
        ErroPerfis::TomlInvalido {
            caminho: PathBuf::from("perfis.toml"),
            detalhe: "quebrado".to_owned(),
        }
    }

    #[test]
    fn lista_as_filas_fornecidas_e_termina_nas_duas_simuladas() {
        let lista = montar(
            Ok(vec![fila("Laser", 0), fila("Jato", OFFLINE)]),
            Ok(Perfis::new()),
        );
        let nomes: Vec<&str> = lista.impressoras.iter().map(|i| i.nome.as_str()).collect();
        assert_eq!(
            nomes,
            ["Laser", "Jato", "Simulada — Traseira", "Simulada — Gaveta"]
        );
        assert!(lista.avisos.is_empty());
        assert_eq!(lista.impressoras[0].nivel, Nivel::Pronta);
        assert!(lista.impressoras[0].condicoes.is_empty());
        assert_eq!(lista.impressoras[1].nivel, Nivel::Problema);
        assert_eq!(lista.impressoras[1].condicoes[0].codigo, "offline");
    }

    #[test]
    fn simuladas_tem_o_modo_fixo_que_o_nome_diz() {
        let lista = montar(Ok(Vec::new()), Ok(Perfis::new()));
        assert_eq!(lista.impressoras.len(), 2);
        assert_eq!(lista.impressoras[0].modo, Modo::Jato);
        assert_eq!(lista.impressoras[0].tipo, Tipo::Simulada);
        assert_eq!(lista.impressoras[1].modo, Modo::Laser);
        assert_eq!(lista.impressoras[1].tipo, Tipo::Simulada);
    }

    #[test]
    fn perfil_vale_so_para_o_nome_exato_da_fila() {
        let perfis = perfis_de(&[("Laser", true)]);
        let lista = montar(
            Ok(vec![
                fila("Laser", 0),
                fila("laser", 0),
                fila("Laser ", 0),
                fila("Laser 2", 0),
            ]),
            Ok(perfis),
        );
        let modos: Vec<Modo> = lista.impressoras.iter().take(4).map(|i| i.modo).collect();
        assert_eq!(
            modos,
            [
                Modo::Personalizado,
                Modo::SemPerfil,
                Modo::SemPerfil,
                Modo::SemPerfil
            ]
        );
    }

    #[test]
    fn perfil_sem_modelo_e_personalizado_seja_qual_for_a_primeira_passada() {
        let perfis = perfis_de(&[("Cima", true), ("Baixo", false)]);
        let lista = montar(Ok(vec![fila("Cima", 0), fila("Baixo", 0)]), Ok(perfis));
        assert_eq!(lista.impressoras[0].modo, Modo::Personalizado);
        assert_eq!(lista.impressoras[1].modo, Modo::Personalizado);
    }

    #[test]
    fn frente_invertida_sozinha_nao_diz_qual_e_o_modo() {
        let perfis = perfis_de(&[("Fila", true)]);
        let lista = montar(Ok(vec![fila("Fila", 0)]), Ok(perfis));
        assert_ne!(lista.impressoras[0].modo, Modo::Jato);
    }

    #[test]
    fn o_modo_vem_do_modelo_salvo_mesmo_com_a_frente_invertida_nos_dois() {
        let mut perfis = Perfis::new();
        perfis.definir_modelo("Jato", ModeloImpressora::JatoDeTinta);
        perfis.definir_modelo("Laser", ModeloImpressora::Laser);
        let lista = montar(Ok(vec![fila("Jato", 0), fila("Laser", 0)]), Ok(perfis));
        assert_eq!(lista.impressoras[0].modo, Modo::Jato);
        assert_eq!(lista.impressoras[1].modo, Modo::Laser);
    }

    /// Pasta temporaria exclusiva do teste, removida ao fim.
    struct Pasta(PathBuf);

    impl Pasta {
        fn nova(nome: &str) -> Self {
            let caminho =
                std::env::temp_dir().join(format!("plots-app-teste-{nome}-{}", std::process::id()));
            let _ = std::fs::remove_dir_all(&caminho);
            std::fs::create_dir_all(&caminho).unwrap();
            Self(caminho)
        }
    }

    impl Drop for Pasta {
        fn drop(&mut self) {
            let _ = std::fs::remove_dir_all(&self.0);
        }
    }

    #[test]
    fn gravar_modelo_mantem_o_apelido_e_aparece_na_lista() {
        let pasta = Pasta::nova("gravar");
        let caminho = pasta.0.join("perfis.toml");
        let mut perfis = Perfis::new();
        perfis.inserir(
            "Fila",
            EntradaPerfil {
                apelido: Some("Sala".to_owned()),
                ..EntradaPerfil::default()
            },
        );
        plots_perfis::salvar(&caminho, &perfis).unwrap();
        gravar_modelo(Ok(caminho.clone()), "Fila", "laser").unwrap();
        gravar_modelo(Ok(caminho.clone()), "Nova", "jato").unwrap();
        let lidos = plots_perfis::carregar(&caminho).unwrap();
        assert_eq!(
            lidos.obter("Fila").unwrap().apelido.as_deref(),
            Some("Sala")
        );
        let lista = montar(Ok(vec![fila("Fila", 0), fila("Nova", 0)]), Ok(lidos));
        assert_eq!(lista.impressoras[0].modo, Modo::Laser);
        assert_eq!(lista.impressoras[1].modo, Modo::Jato);
    }

    #[test]
    fn falha_ao_gravar_diz_a_fila_e_o_arquivo_e_nao_muda_nada() {
        let pasta = Pasta::nova("falha");
        // A pasta do arquivo e um arquivo: nao ha como criar nada ali dentro.
        let bloqueio = pasta.0.join("bloqueio");
        std::fs::write(&bloqueio, "x").unwrap();
        let caminho = bloqueio.join("perfis.toml");
        let aviso = gravar_modelo(Ok(caminho.clone()), "Fila", "jato").unwrap_err();
        assert_eq!(aviso.codigo, CodigoAviso::ModeloNaoGravado);
        assert_eq!(aviso.detalhe.as_deref(), Some("Fila"));
        assert_eq!(aviso.caminho, Some(caminho.display().to_string()));
        assert_eq!(std::fs::read_to_string(&bloqueio).unwrap(), "x");
    }

    #[test]
    fn arquivo_danificado_nao_e_sobrescrito_pela_escolha_do_modelo() {
        let pasta = Pasta::nova("danificado");
        let caminho = pasta.0.join("perfis.toml");
        std::fs::write(&caminho, "nada = = =").unwrap();
        let aviso = gravar_modelo(Ok(caminho.clone()), "Fila", "jato").unwrap_err();
        assert_eq!(aviso.codigo, CodigoAviso::ModeloNaoGravado);
        assert_eq!(std::fs::read_to_string(&caminho).unwrap(), "nada = = =");
    }

    #[test]
    fn modelo_desconhecido_ou_fila_sem_nome_nao_tocam_no_arquivo() {
        let pasta = Pasta::nova("recusas");
        let caminho = pasta.0.join("perfis.toml");
        for (fila, codigo) in [
            ("Fila", "matricial"),
            ("Fila", ""),
            ("", "jato"),
            ("  ", "laser"),
        ] {
            let aviso = gravar_modelo(Ok(caminho.clone()), fila, codigo).unwrap_err();
            assert_eq!(aviso.codigo, CodigoAviso::ModeloNaoGravado);
            assert!(!caminho.exists());
        }
    }

    #[test]
    fn sem_pasta_do_usuario_a_escolha_vira_aviso_sem_caminho() {
        let aviso =
            gravar_modelo(Err(ErroPerfis::PastaUsuarioIndisponivel), "Fila", "jato").unwrap_err();
        assert_eq!(aviso.codigo, CodigoAviso::ModeloNaoGravado);
        assert_eq!(aviso.caminho, None);
    }

    #[test]
    fn falha_de_perfis_vira_aviso_e_nao_diz_que_a_fila_nao_tem_perfil() {
        let lista = montar(Ok(vec![fila("Laser", 0)]), Err(falha_de_toml()));
        assert_eq!(lista.impressoras.len(), 3);
        assert_eq!(lista.impressoras[0].modo, Modo::Desconhecido);
        assert_eq!(lista.impressoras[1].modo, Modo::Jato);
        assert_eq!(
            lista.avisos,
            [Aviso {
                codigo: CodigoAviso::PerfisInvalido,
                detalhe: None,
                caminho: Some("perfis.toml".to_owned()),
            }]
        );
    }

    #[test]
    fn cada_falha_de_perfis_tem_o_seu_codigo() {
        let caminho = PathBuf::from("perfis.toml");
        let casos = [
            (
                ErroPerfis::PastaUsuarioIndisponivel,
                CodigoAviso::PerfisPasta,
            ),
            (
                ErroPerfis::Leitura {
                    caminho: caminho.clone(),
                    origem: std::io::Error::other("recusado"),
                },
                CodigoAviso::PerfisLeitura,
            ),
            (
                ErroPerfis::ArquivoGrande {
                    caminho: caminho.clone(),
                },
                CodigoAviso::PerfisGrande,
            ),
            (
                ErroPerfis::VersaoDesconhecida {
                    caminho,
                    versao: 99,
                },
                CodigoAviso::PerfisVersao,
            ),
        ];
        for (erro, esperado) in casos {
            assert_eq!(codigo_de_perfis(&erro), esperado);
        }
    }

    #[test]
    fn falha_nas_filas_mantem_as_simuladas_e_mostra_o_codigo() {
        let lista = montar(Err(ErroFilas::Sistema(0x8007_06BA)), Ok(Perfis::new()));
        assert_eq!(lista.impressoras.len(), 2);
        assert_eq!(
            lista.avisos,
            [Aviso {
                codigo: CodigoAviso::Filas,
                detalhe: Some("0x800706BA".to_owned()),
                caminho: None,
            }]
        );
    }

    #[test]
    fn aviso_de_perfis_diz_qual_arquivo_quando_ha_um() {
        let caminho = PathBuf::from("C:\\perfis\\perfis.toml");
        let lista = montar(
            Ok(Vec::new()),
            Err(ErroPerfis::ArquivoGrande {
                caminho: caminho.clone(),
            }),
        );
        assert_eq!(
            lista.avisos[0].caminho.as_deref(),
            Some("C:\\perfis\\perfis.toml")
        );
        let sem_pasta = montar(Ok(Vec::new()), Err(ErroPerfis::PastaUsuarioIndisponivel));
        assert_eq!(sem_pasta.avisos[0].caminho, None);
    }

    #[test]
    fn fora_do_windows_so_as_simuladas_e_sem_aviso() {
        let lista = montar(Err(ErroFilas::NaoSuportado), Ok(Perfis::new()));
        assert_eq!(lista.impressoras.len(), 2);
        assert!(lista.avisos.is_empty());
    }

    #[test]
    fn as_duas_falhas_juntas_nao_derrubam_a_lista() {
        let lista = montar(
            Err(ErroFilas::Resposta("forma estranha")),
            Err(falha_de_toml()),
        );
        assert_eq!(lista.impressoras.len(), 2);
        assert_eq!(lista.avisos.len(), 2);
    }

    #[test]
    fn leitura_interrompida_mantem_as_simuladas_e_avisa() {
        let lista = sem_leitura();
        assert_eq!(lista.impressoras.len(), 2);
        assert_eq!(lista.avisos[0].codigo, CodigoAviso::Filas);
    }

    #[test]
    fn fila_real_com_nome_de_simulada_nao_se_confunde_com_ela() {
        let lista = montar(Ok(vec![fila("Simulada — Gaveta", 0)]), Ok(Perfis::new()));
        let ids: HashSet<&str> = lista.impressoras.iter().map(|i| i.id.as_str()).collect();
        assert_eq!(ids.len(), 3);
        assert_eq!(lista.impressoras[0].tipo, Tipo::Fila);
        assert_eq!(lista.impressoras[0].modo, Modo::SemPerfil);
    }

    #[test]
    fn nome_repetido_pelo_sistema_aparece_uma_vez() {
        let lista = montar(
            Ok(vec![fila("A", 0), fila("A", OFFLINE)]),
            Ok(Perfis::new()),
        );
        assert_eq!(lista.impressoras.len(), 3);
        assert_eq!(lista.impressoras[0].nivel, Nivel::Pronta);
    }

    #[test]
    fn o_nivel_e_o_da_condicao_mais_grave() {
        let lista = montar(
            Ok(vec![
                fila("a", IMPRIMINDO),
                fila("b", IMPRIMINDO | POUCO_TONER),
                fila("c", IMPRIMINDO | POUCO_TONER | OFFLINE),
            ]),
            Ok(Perfis::new()),
        );
        assert_eq!(lista.impressoras[0].nivel, Nivel::Ativa);
        assert_eq!(lista.impressoras[1].nivel, Nivel::Atencao);
        assert_eq!(lista.impressoras[2].nivel, Nivel::Problema);
    }

    #[test]
    fn cada_bit_vira_um_codigo_distinto_e_nenhum_se_perde() {
        let mut codigos = HashSet::new();
        for posicao in 0..27 {
            let estado = Estado::de_bits(1u32 << posicao);
            let [condicao] = estado.condicoes() else {
                panic!("o bit {posicao} deveria dar uma condicao");
            };
            let (lida, _) = ler_condicao(condicao);
            assert!(
                codigos.insert(lida.codigo),
                "codigo repetido: {}",
                lida.codigo
            );
        }
        assert_eq!(codigos.len(), 27);
    }

    #[test]
    fn alimentacao_manual_tem_codigo_proprio_e_pede_atencao() {
        let lista = montar(Ok(vec![fila("a", 0x0000_0020)]), Ok(Perfis::new()));
        assert_eq!(
            lista.impressoras[0].condicoes[0].codigo,
            "alimentacao_manual"
        );
        assert_eq!(lista.impressoras[0].nivel, Nivel::Atencao);
    }

    #[test]
    fn bit_sem_traducao_segue_com_o_codigo_exato() {
        let lista = montar(Ok(vec![fila("a", 0x0800_0000)]), Ok(Perfis::new()));
        assert_eq!(
            lista.impressoras[0].condicoes,
            [CondicaoLida {
                codigo: "outro",
                detalhe: Some("0x08000000".to_owned()),
            }]
        );
    }

    #[test]
    fn variavel_de_perfis_troca_o_caminho_e_vazia_nao_conta() {
        let caminho = caminho_dos_perfis(Some(OsString::from("C:\\teste\\perfis.toml"))).unwrap();
        assert_eq!(caminho, Path::new("C:\\teste\\perfis.toml"));
        if let Ok(padrao) = plots_perfis::caminho_padrao() {
            assert_eq!(caminho_dos_perfis(Some(OsString::new())).unwrap(), padrao);
            assert_eq!(caminho_dos_perfis(None).unwrap(), padrao);
        }
    }
}
