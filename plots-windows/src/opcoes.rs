//! Opções que o driver da fila declara e que o envio não fixa: bandeja, resolução e cor.
//!
//! O `DEVMODE` continua sendo a fonte dos valores atuais. Aqui ficam os tipos públicos e a
//! interpretação das respostas do driver, longe das chamadas do sistema, para que os arrays
//! hostis sejam conferidos sem tocar em uma impressora.

use crate::erro::ErroEnvio;
#[cfg(not(windows))]
use crate::preferencias::ErroPreferencias;

/// Cor que o driver aceita para a fila.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Cor {
    Colorida,
    Monocromatica,
}

/// Bandeja de entrada declarada pelo driver.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Bandeja {
    /// Código que o driver usa no `DEVMODE` (`dmDefaultSource`).
    pub valor: i16,
    /// Nome que o driver dá à bandeja, sem caracteres de controle.
    pub nome: String,
}

/// Par de resolução declarado pelo driver, em dpi.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct Resolucao {
    pub x: i32,
    pub y: i32,
}

/// O que a pessoa escolheu no painel. `None` deixa o campo como o driver o entregou.
#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct Escolhas {
    pub bandeja: Option<i16>,
    pub resolucao: Option<(i32, i32)>,
    pub cor: Option<Cor>,
}

/// O que o driver declara para uma fila, com os valores que o ajuste atual usa.
#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct OpcoesDaFila {
    pub bandejas: Vec<Bandeja>,
    pub resolucoes: Vec<Resolucao>,
    pub cor: Option<Cor>,
    pub atual: Escolhas,
}

/// Quantos itens o driver pode declarar antes de a resposta ser tratada como absurda.
pub(crate) const MAXIMO_DE_OPCOES: i32 = 256;

/// Nome de bandeja que o `DEVMODE` e o `DeviceCapabilities` usam, em unidades UTF-16.
pub(crate) const CARACTERES_DO_NOME_DA_BANDEJA: usize = 24;

/// `DeviceCapabilities` devolve a quantidade de itens, zero quando não há e um valor negativo
/// quando não conseguiu responder. Um número fora do limite é resposta corrompida, não lista.
pub(crate) fn total_declarado(resposta: i32) -> Result<usize, ErroEnvio> {
    if resposta <= 0 {
        return Ok(0);
    }
    if resposta > MAXIMO_DE_OPCOES {
        return Err(ErroEnvio::Resposta("o driver declarou opções demais"));
    }
    Ok(resposta as usize)
}

/// Junta os códigos de bandeja aos nomes, na ordem em que o driver os devolveu. Um nome vazio
/// não vira opção: o painel não inventa rótulo para o que o driver não nomeou.
pub(crate) fn bandejas_de(valores: &[i16], nomes: &[u16]) -> Vec<Bandeja> {
    let mut vistos = std::collections::BTreeSet::new();
    let mut bandejas = Vec::new();
    for (indice, valor) in valores.iter().enumerate() {
        if !vistos.insert(*valor) {
            continue;
        }
        let Some(inicio) = indice.checked_mul(CARACTERES_DO_NOME_DA_BANDEJA) else {
            continue;
        };
        let Some(faixa) = nomes.get(inicio..inicio.saturating_add(CARACTERES_DO_NOME_DA_BANDEJA))
        else {
            continue;
        };
        let fim = faixa
            .iter()
            .position(|unidade| *unidade == 0)
            .unwrap_or(faixa.len());
        let nome = String::from_utf16_lossy(&faixa[..fim]);
        let nome = nome.trim();
        if nome.is_empty() {
            continue;
        }
        bandejas.push(Bandeja {
            valor: *valor,
            nome: nome.to_owned(),
        });
    }
    bandejas
}

/// Lê os pares `(x, y)` de `DC_ENUMRESOLUTIONS`. Um par incompleto ou sem os dois lados
/// positivos não é uma resolução que se possa pedir ao driver.
pub(crate) fn resolucoes_de(pares: &[i32]) -> Vec<Resolucao> {
    let mut vistos = std::collections::BTreeSet::new();
    pares
        .chunks_exact(2)
        .filter_map(|par| {
            let (x, y) = (par[0], par[1]);
            (x > 0 && y > 0 && vistos.insert((x, y))).then_some(Resolucao { x, y })
        })
        .collect()
}

/// `DC_COLORDEVICE` responde 1 para uma impressora que imprime em cores e 2 para uma que só
/// imprime em preto. Qualquer outra resposta não afirma nada sobre a cor.
pub(crate) fn cor_de(resposta: i32) -> Option<Cor> {
    match resposta {
        1 => Some(Cor::Colorida),
        2 => Some(Cor::Monocromatica),
        _ => None,
    }
}

/// Fora do Windows não há driver a consultar.
#[cfg(not(windows))]
pub fn opcoes_da_fila(
    _fila: &str,
    _salvo: Option<&[u8]>,
) -> Result<OpcoesDaFila, ErroPreferencias> {
    Err(ErroPreferencias::Envio(ErroEnvio::NaoSuportado))
}

/// Fora do Windows não há `DEVMODE` a montar.
#[cfg(not(windows))]
pub fn aplicar_opcoes_da_fila(
    _fila: &str,
    _salvo: Option<&[u8]>,
    _escolhas: &Escolhas,
) -> Result<Vec<u8>, ErroPreferencias> {
    Err(ErroPreferencias::Envio(ErroEnvio::NaoSuportado))
}

#[cfg(test)]
mod testes {
    use super::*;

    fn nomes_de(lista: &[&str]) -> Vec<u16> {
        let mut nomes = Vec::new();
        for nome in lista {
            let mut unidades: Vec<u16> = nome.encode_utf16().collect();
            unidades.resize(CARACTERES_DO_NOME_DA_BANDEJA, 0);
            nomes.extend(unidades);
        }
        nomes
    }

    #[test]
    fn bandejas_juntam_codigo_e_nome_na_ordem_do_driver() {
        let nomes = nomes_de(&["Bandeja 1", "Bandeja 2"]);
        assert_eq!(
            bandejas_de(&[1, 2], &nomes),
            [
                Bandeja {
                    valor: 1,
                    nome: "Bandeja 1".into()
                },
                Bandeja {
                    valor: 2,
                    nome: "Bandeja 2".into()
                }
            ]
        );
    }

    #[test]
    fn cada_nome_de_bandeja_ocupa_vinte_e_quatro_unidades() {
        let mut nomes = vec![0u16; 4 * CARACTERES_DO_NOME_DA_BANDEJA];
        for (indice, letra) in [b'A', b'B', b'C', b'D'].iter().enumerate() {
            nomes[indice * CARACTERES_DO_NOME_DA_BANDEJA] = u16::from(*letra);
        }
        let bandejas = bandejas_de(&[1, 2, 3, 4], &nomes);
        let nomes: Vec<&str> = bandejas
            .iter()
            .map(|bandeja| bandeja.nome.as_str())
            .collect();
        assert_eq!(nomes, ["A", "B", "C", "D"]);
    }

    #[test]
    fn bandeja_sem_nome_nao_vira_opcao_e_codigo_repetido_aparece_uma_vez() {
        let nomes = nomes_de(&["", "Bandeja 2", "Bandeja 2"]);
        assert_eq!(
            bandejas_de(&[1, 2, 2], &nomes),
            [Bandeja {
                valor: 2,
                nome: "Bandeja 2".into()
            }]
        );
    }

    #[test]
    fn nome_sem_zero_final_nao_estoura_o_buffer() {
        let mut nomes = vec![u16::from(b'X'); CARACTERES_DO_NOME_DA_BANDEJA];
        nomes[0] = u16::from(b'O');
        let bandejas = bandejas_de(&[9], &nomes);
        assert_eq!(bandejas[0].nome.len(), CARACTERES_DO_NOME_DA_BANDEJA);
    }

    #[test]
    fn resolucoes_saem_aos_pares_e_o_que_nao_e_par_fica_de_fora() {
        assert_eq!(
            resolucoes_de(&[600, 600, 300, 300, 1200]),
            [Resolucao { x: 600, y: 600 }, Resolucao { x: 300, y: 300 }]
        );
    }

    #[test]
    fn resolucao_com_lado_nao_positivo_e_repetida_nao_entra() {
        assert_eq!(
            resolucoes_de(&[0, 600, 600, 0, -1, 300, 600, 600]),
            [Resolucao { x: 600, y: 600 }]
        );
    }

    #[test]
    fn cor_so_aceita_as_duas_respostas_conhecidas() {
        assert_eq!(cor_de(1), Some(Cor::Colorida));
        assert_eq!(cor_de(2), Some(Cor::Monocromatica));
        for resposta in [0, 3, -1, i32::MIN, i32::MAX] {
            assert_eq!(cor_de(resposta), None);
        }
    }

    #[test]
    fn sem_resposta_do_driver_as_listas_ficam_vazias() {
        assert_eq!(total_declarado(-1), Ok(0));
        assert_eq!(total_declarado(0), Ok(0));
        assert!(bandejas_de(&[], &[]).is_empty());
        assert!(resolucoes_de(&[]).is_empty());
        assert_eq!(cor_de(-1), None);
    }

    #[test]
    fn quantidade_absurda_declarada_pelo_driver_nao_reserva_memoria() {
        assert_eq!(
            total_declarado(MAXIMO_DE_OPCOES),
            Ok(MAXIMO_DE_OPCOES as usize)
        );
        for resposta in [MAXIMO_DE_OPCOES + 1, 100_000, i32::MAX] {
            assert!(total_declarado(resposta).is_err());
        }
    }
}
