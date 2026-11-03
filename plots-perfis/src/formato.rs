use std::collections::BTreeMap;
use std::path::Path;

use plots_core::distribuicao::ModoDistribuicao;
use plots_core::{BordaEntrada, GestoRecolocar, ModeloImpressora, PerfilImpressora, ViradaMaco};
use serde::{Deserialize, Serialize};

use crate::ajustes;
use crate::erro::ErroPerfis;
use crate::grupos::{Grupo, GrupoIgnorado};
use crate::perfis::{EntradaPerfil, Perfis};

/// Versão do formato gravado em `perfis.toml`.
pub const VERSAO_ATUAL: u32 = 1;

/// Lê só a versão, ignorando o resto, para recusar formatos futuros com mensagem clara.
#[derive(Deserialize)]
struct Cabecalho {
    versao: u32,
}

#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct ArquivoPerfis {
    versao: u32,
    #[serde(default)]
    filas: BTreeMap<String, EntradaArquivo>,
    /// Ausente em arquivos antigos; sem grupos, o arquivo gravado fica igual ao de antes. Valor
    /// solto: um grupo de formato errado é ignorado sem recusar os perfis.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    grupos: Option<toml::Value>,
    /// Ajuste do driver por fila. Fica como valor solto: um valor ruim não recusa o arquivo.
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    ajustes_driver: BTreeMap<String, toml::Value>,
}

#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct GrupoArquivo {
    impressoras: Vec<String>,
    /// Valor solto: um modo desconhecido vale como espelho, que não reparte nada, sem recusar o
    /// arquivo inteiro.
    modo: toml::Value,
}

const MODO_ESPELHO: &str = "espelho";
const MODO_DIVISAO: &str = "divisao";

impl From<GrupoArquivo> for Grupo {
    fn from(grupo: GrupoArquivo) -> Self {
        Self {
            impressoras: grupo.impressoras,
            modo: match grupo.modo.as_str() {
                Some(MODO_DIVISAO) => ModoDistribuicao::Divisao,
                _ => ModoDistribuicao::Espelho,
            },
        }
    }
}

impl From<&Grupo> for GrupoArquivo {
    fn from(grupo: &Grupo) -> Self {
        Self {
            impressoras: grupo.impressoras.clone(),
            modo: toml::Value::String(
                match grupo.modo {
                    ModoDistribuicao::Espelho => MODO_ESPELHO,
                    ModoDistribuicao::Divisao => MODO_DIVISAO,
                }
                .to_owned(),
            ),
        }
    }
}

#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct EntradaArquivo {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    apelido: Option<String>,
    passada1_inversa: bool,
    passada2_inversa: bool,
    verso_girar_180: bool,
    virar_maco: bool,
    /// Fica como valor solto, como `borda_virada` e `modelo`: um valor inválido não recusa o
    /// arquivo inteiro.
    borda_primeiro: toml::Value,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    gesto_texto: Option<String>,
    /// Por onde o maço vira; ausente, vira pela lateral. Só faz sentido com `virar_maco`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    borda_virada: Option<toml::Value>,
    /// Código do modelo de que os valores vieram; ausente, o perfil foi medido ou escrito à mão.
    /// Fica como valor solto para que um tipo errado não recuse o arquivo inteiro.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    modelo: Option<toml::Value>,
}

const VIRADA_LATERAL: &str = "lateral";
const VIRADA_CIMA: &str = "cima";
const BORDA_CURTA: &str = "curta";
const BORDA_LONGA: &str = "longa";

fn virada_lida(valor: &toml::Value) -> Option<ViradaMaco> {
    match valor.as_str()? {
        VIRADA_LATERAL => Some(ViradaMaco::PelaLateral),
        VIRADA_CIMA => Some(ViradaMaco::PelaBordaDeCima),
        _ => None,
    }
}

fn borda_lida(valor: &toml::Value) -> Option<BordaEntrada> {
    match valor.as_str()? {
        BORDA_CURTA => Some(BordaEntrada::Curta),
        BORDA_LONGA => Some(BordaEntrada::Longa),
        _ => None,
    }
}

/// Um perfil que se diz do modelo precisa ter os valores dele: a interface mostraria um modelo
/// e a impressão seguiria outros valores.
fn modelo_confere(entrada: &EntradaPerfil) -> bool {
    entrada
        .modelo
        .is_none_or(|modelo| modelo.perfil() == entrada.perfil)
}

/// Lê a entrada sem recusar o arquivo por causa dela: o modelo que não se sustenta some e os
/// valores gravados ficam, como em um perfil personalizado. Uma borda inválida também tira o modelo
/// e vale como a borda curta ou a virada pela lateral. As outras filas não são afetadas.
impl From<EntradaArquivo> for EntradaPerfil {
    fn from(entrada: EntradaArquivo) -> Self {
        let virada_gravada = match &entrada.borda_virada {
            None => Some(ViradaMaco::PelaLateral),
            Some(valor) => virada_lida(valor),
        };
        let borda_gravada = borda_lida(&entrada.borda_primeiro);
        let bordas_validas = virada_gravada.is_some() && borda_gravada.is_some();
        let virada = if entrada.virar_maco {
            virada_gravada.unwrap_or(ViradaMaco::PelaLateral)
        } else {
            ViradaMaco::NaoVirar
        };
        let perfil = PerfilImpressora {
            passada1_inversa: entrada.passada1_inversa,
            passada2_inversa: entrada.passada2_inversa,
            verso_girar_180: entrada.verso_girar_180,
            gesto_recolocar: GestoRecolocar {
                virada,
                borda_primeiro: borda_gravada.unwrap_or(BordaEntrada::Curta),
            },
        };
        let modelo = entrada
            .modelo
            .as_ref()
            .and_then(toml::Value::as_str)
            .and_then(ModeloImpressora::de_codigo)
            .filter(|modelo| bordas_validas && modelo.perfil() == perfil);
        EntradaPerfil {
            apelido: entrada.apelido,
            perfil,
            gesto_texto: entrada.gesto_texto,
            modelo,
        }
    }
}

impl From<&EntradaPerfil> for EntradaArquivo {
    fn from(entrada: &EntradaPerfil) -> Self {
        let perfil = entrada.perfil;
        EntradaArquivo {
            apelido: entrada.apelido.clone(),
            passada1_inversa: perfil.passada1_inversa,
            passada2_inversa: perfil.passada2_inversa,
            verso_girar_180: perfil.verso_girar_180,
            virar_maco: perfil.gesto_recolocar.vira_o_maco(),
            borda_primeiro: toml::Value::String(
                match perfil.gesto_recolocar.borda_primeiro {
                    BordaEntrada::Curta => BORDA_CURTA,
                    BordaEntrada::Longa => BORDA_LONGA,
                }
                .to_owned(),
            ),
            gesto_texto: entrada.gesto_texto.clone(),
            borda_virada: match perfil.gesto_recolocar.virada {
                ViradaMaco::PelaBordaDeCima => Some(toml::Value::String(VIRADA_CIMA.to_owned())),
                ViradaMaco::NaoVirar | ViradaMaco::PelaLateral => None,
            },
            modelo: entrada
                .modelo
                .map(|modelo| toml::Value::String(modelo.codigo().to_owned())),
        }
    }
}

/// A chave usada quando `grupos` nem é uma tabela. Nenhum nome válido é vazio, e o valor fica guardado.
const CHAVE_SEM_TABELA: &str = "";

fn ler_grupos(perfis: &mut Perfis, grupos: Option<toml::Value>) {
    let tabela = match grupos {
        None => return,
        Some(toml::Value::Table(tabela)) => tabela,
        Some(outro) => {
            perfis
                .grupos_ignorados
                .insert(CHAVE_SEM_TABELA.to_owned(), GrupoIgnorado(outro));
            return;
        }
    };
    for (nome, valor) in tabela {
        let aceito = valor
            .clone()
            .try_into::<GrupoArquivo>()
            .ok()
            .is_some_and(|grupo| perfis.aceitar_grupo_lido(&nome, grupo.into()).is_ok());
        if !aceito {
            perfis.grupos_ignorados.insert(nome, GrupoIgnorado(valor));
        }
    }
}

fn escrever_grupos(perfis: &Perfis) -> Result<Option<toml::Value>, ErroPerfis> {
    let mut tabela = toml::Table::new();
    for (nome, grupo) in perfis.grupos() {
        let valor = toml::Value::try_from(GrupoArquivo::from(grupo)).map_err(|erro| {
            ErroPerfis::Serializacao {
                detalhe: erro.to_string(),
            }
        })?;
        tabela.insert(nome.to_owned(), valor);
    }
    for (nome, ignorado) in &perfis.grupos_ignorados {
        tabela.insert(nome.clone(), ignorado.0.clone());
    }
    Ok((!tabela.is_empty()).then_some(toml::Value::Table(tabela)))
}

/// Interpreta o texto de `perfis.toml`. Não altera nada em disco.
pub(crate) fn decodificar(texto: &str, caminho: &Path) -> Result<Perfis, ErroPerfis> {
    let cabecalho: Cabecalho = toml::from_str(texto).map_err(|erro| ErroPerfis::TomlInvalido {
        caminho: caminho.to_path_buf(),
        detalhe: erro.to_string(),
    })?;
    if cabecalho.versao != VERSAO_ATUAL {
        return Err(ErroPerfis::VersaoDesconhecida {
            caminho: caminho.to_path_buf(),
            versao: cabecalho.versao,
        });
    }
    let arquivo: ArquivoPerfis =
        toml::from_str(texto).map_err(|erro| ErroPerfis::TomlInvalido {
            caminho: caminho.to_path_buf(),
            detalhe: erro.to_string(),
        })?;
    let mut perfis = Perfis::new();
    for (fila, entrada) in arquivo.filas {
        perfis.inserir(&fila, entrada.into());
    }
    ler_grupos(&mut perfis, arquivo.grupos);
    ajustes::ler_do_arquivo(&mut perfis, arquivo.ajustes_driver);
    Ok(perfis)
}

/// Gera o texto de `perfis.toml` para o conjunto de perfis.
///
/// Recusa uma entrada cujo modelo não confere: a leitura a abriria sem o modelo, e o que foi
/// pedido para gravar se perderia em silêncio.
pub(crate) fn codificar(perfis: &Perfis) -> Result<String, ErroPerfis> {
    if !perfis.filas().all(|(_, entrada)| modelo_confere(entrada)) {
        return Err(ErroPerfis::Serializacao {
            detalhe: "os valores do perfil não são os do modelo indicado".to_owned(),
        });
    }
    let arquivo = ArquivoPerfis {
        versao: VERSAO_ATUAL,
        filas: perfis
            .filas()
            .map(|(fila, entrada)| (fila.to_owned(), EntradaArquivo::from(entrada)))
            .collect(),
        grupos: escrever_grupos(perfis)?,
        ajustes_driver: ajustes::para_o_arquivo(perfis),
    };
    toml::to_string(&arquivo).map_err(|erro| ErroPerfis::Serializacao {
        detalhe: erro.to_string(),
    })
}
