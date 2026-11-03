//! Distribuição de índices de lotes entre filas selecionadas.

use std::collections::TryReserveError;

/// Espelho envia todos os lotes a cada fila; divisão reparte os lotes em rodízio.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ModoDistribuicao {
    Espelho,
    Divisao,
}

/// Associação a uma fila pela posição na seleção fornecida pelo chamador.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Atribuicao {
    pub indice_impressora: usize,
    primeiro: usize,
    total_lotes: usize,
    passo: usize,
}

impl Atribuicao {
    /// Índices de lotes, começando em zero e mantendo a ordem do documento.
    /// A sequência é calculada sob demanda, sem copiar os lotes.
    pub fn indices_lotes(&self) -> impl Iterator<Item = usize> {
        (self.primeiro..self.total_lotes).step_by(self.passo.max(1))
    }

    /// Fica só com os itens desta fila, na ordem do documento.
    pub fn selecionar<T>(&self, itens: Vec<T>) -> Vec<T> {
        itens
            .into_iter()
            .take(self.total_lotes)
            .skip(self.primeiro)
            .step_by(self.passo.max(1))
            .collect()
    }
}

/// Na divisão, só as primeiras filas recebem quando há mais filas do que lotes: o rodízio dá um
/// lote a cada uma, na ordem da seleção, e as demais ficam sem nada.
pub const fn filas_com_lote(total_lotes: usize, total_impressoras: usize) -> usize {
    if total_lotes < total_impressoras {
        total_lotes
    } else {
        total_impressoras
    }
}

#[derive(Debug)]
pub enum ErroDistribuicao {
    NenhumaImpressora,
    LimiteImpressorasExcedido,
    ReservaFalhou(TryReserveError),
}

impl std::fmt::Display for ErroDistribuicao {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::NenhumaImpressora => f.write_str("selecione ao menos uma impressora"),
            Self::LimiteImpressorasExcedido => {
                f.write_str("quantidade de impressoras excede o limite")
            }
            Self::ReservaFalhou(_) => f.write_str("não foi possível reservar a distribuição"),
        }
    }
}

impl std::error::Error for ErroDistribuicao {}

/// Planeja os índices por fila. Cada lote deve ser planejado novamente com o perfil
/// da fila correspondente. Na divisão, as quantidades diferem em no máximo um lote.
pub fn distribuir_lotes(
    total_lotes: usize,
    total_impressoras: usize,
    modo: ModoDistribuicao,
) -> Result<Vec<Atribuicao>, ErroDistribuicao> {
    if total_impressoras == 0 {
        return Err(ErroDistribuicao::NenhumaImpressora);
    }
    if total_impressoras > crate::LIMITE_PAGINAS {
        return Err(ErroDistribuicao::LimiteImpressorasExcedido);
    }
    let mut resultado = Vec::new();
    resultado
        .try_reserve_exact(total_impressoras)
        .map_err(ErroDistribuicao::ReservaFalhou)?;
    for indice_impressora in 0..total_impressoras {
        let (primeiro, passo) = match modo {
            ModoDistribuicao::Espelho => (0, 1),
            ModoDistribuicao::Divisao => (indice_impressora, total_impressoras),
        };
        resultado.push(Atribuicao {
            indice_impressora,
            primeiro,
            total_lotes,
            passo,
        });
    }
    Ok(resultado)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn espelho_preserva_todos_os_indices_em_cada_fila() {
        let plano = distribuir_lotes(5, 2, ModoDistribuicao::Espelho).unwrap();
        assert_eq!(plano[0].indice_impressora, 0);
        assert_eq!(plano[1].indice_impressora, 1);
        for fila in plano {
            assert_eq!(
                fila.indices_lotes().collect::<Vec<_>>(),
                vec![0, 1, 2, 3, 4]
            );
        }
    }

    #[test]
    fn divisao_reparte_sem_repeticoes_e_mantem_ordem() {
        let plano = distribuir_lotes(8, 3, ModoDistribuicao::Divisao).unwrap();
        let indices: Vec<Vec<_>> = plano.iter().map(|p| p.indices_lotes().collect()).collect();
        assert_eq!(indices, vec![vec![0, 3, 6], vec![1, 4, 7], vec![2, 5]]);
    }

    #[test]
    fn selecionar_devolve_os_itens_de_cada_fila_uma_vez_so() {
        for total_lotes in 0..40 {
            for total_impressoras in 1..9 {
                let plano =
                    distribuir_lotes(total_lotes, total_impressoras, ModoDistribuicao::Divisao)
                        .unwrap();
                let mut vistos = Vec::new();
                for fila in &plano {
                    let itens: Vec<usize> = (0..total_lotes).collect();
                    let escolhidos = fila.selecionar(itens);
                    assert_eq!(
                        escolhidos,
                        fila.indices_lotes().collect::<Vec<_>>(),
                        "a seleção segue os índices da fila"
                    );
                    vistos.extend(escolhidos);
                }
                vistos.sort_unstable();
                assert_eq!(vistos, (0..total_lotes).collect::<Vec<_>>());
                assert_eq!(
                    plano
                        .iter()
                        .filter(|fila| fila.indices_lotes().next().is_some())
                        .count(),
                    filas_com_lote(total_lotes, total_impressoras)
                );
            }
        }
    }

    #[test]
    fn selecionar_no_espelho_mantem_tudo_e_ignora_o_que_sobra() {
        let plano = distribuir_lotes(3, 2, ModoDistribuicao::Espelho).unwrap();
        assert_eq!(plano[1].selecionar(vec![10, 20, 30, 40]), vec![10, 20, 30]);
        assert_eq!(plano[1].selecionar(vec![10]), vec![10]);
    }

    #[test]
    fn zero_lotes_e_mais_filas_que_lotes() {
        for modo in [ModoDistribuicao::Espelho, ModoDistribuicao::Divisao] {
            let vazio = distribuir_lotes(0, 3, modo).unwrap();
            assert!(vazio.iter().all(|p| p.indices_lotes().next().is_none()));
        }
        let plano = distribuir_lotes(1, 3, ModoDistribuicao::Divisao).unwrap();
        assert_eq!(plano[0].indices_lotes().collect::<Vec<_>>(), vec![0]);
        assert!(plano[1].indices_lotes().next().is_none());
        assert!(plano[2].indices_lotes().next().is_none());
        assert!(matches!(
            distribuir_lotes(0, 0, ModoDistribuicao::Espelho),
            Err(ErroDistribuicao::NenhumaImpressora)
        ));
        assert!(matches!(
            distribuir_lotes(0, usize::MAX, ModoDistribuicao::Divisao),
            Err(ErroDistribuicao::LimiteImpressorasExcedido)
        ));
    }
}
