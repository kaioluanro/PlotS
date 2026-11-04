//! Opções do envio manual que mudam o que vai à fila, sem mudar o plano: a marca de controle no
//! verso e a compensação de um driver que imprime cada trabalho na ordem contrária. Um lote de duas
//! faces fica de fora das duas: sai como o documento, sem marca e sem inverter.

use crate::fluxo::{DescreveFalha, LoteDoFluxo};
use crate::job::{Backend, EstadoEnvio, Passada};
use plots_core::{IdentidadeDaPagina, Pagina};
use std::collections::BTreeMap;

/// Em que idioma a marca sai no papel.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum IdiomaDaMarca {
    Portugues,
    Ingles,
}

impl IdiomaDaMarca {
    fn texto(self, folha: usize) -> String {
        match self {
            Self::Portugues => format!("Folha {folha}"),
            Self::Ingles => format!("Sheet {folha}"),
        }
    }
}

/// A marca de cada folha de um lote, na face de trás dela: a segunda, a página par ou a branca do
/// fim. É a que sai na primeira passada e fica à vista na hora de recolocar o maço. `antes` diz quantas
/// folhas do documento vêm antes deste lote; a ordem e a quantidade de folhas não mudam.
fn marcas_do_lote(
    item: &LoteDoFluxo,
    antes: usize,
    idioma: IdiomaDaMarca,
) -> BTreeMap<IdentidadeDaPagina, String> {
    item.lote
        .paginas
        .chunks(2)
        .enumerate()
        .map(|(indice, folha)| {
            let verso = folha.get(1).map(Pagina::identidade);
            (
                verso.unwrap_or_default(),
                idioma.texto(antes.saturating_add(indice).saturating_add(1)),
            )
        })
        .collect()
}

/// Marca os versos dos lotes manuais. Os lotes têm de ser os do documento inteiro, antes de a divisão
/// repartir: as folhas são contadas no documento, e o número não se repete entre as impressoras.
pub fn marcar_lotes(lotes: &mut [LoteDoFluxo], idioma: IdiomaDaMarca) {
    let mut antes = 0usize;
    for item in lotes.iter_mut() {
        if !item.lote.duas_faces {
            item.marcas = marcas_do_lote(item, antes, idioma);
        }
        antes = antes.saturating_add(item.lote.paginas.len().div_ceil(2));
    }
}

/// As duas passadas de cada lote manual vão na ordem contrária à do plano. Depois de o driver
/// inverter, a impressora recebe a ordem certa. A marca e o papel seguem a página, não a posição.
pub fn compensar_driver(lotes: &mut [LoteDoFluxo]) {
    for item in lotes.iter_mut().filter(|item| !item.lote.duas_faces) {
        item.lote.passada1.reverse();
        item.lote.passada2.reverse();
    }
}

/// Faz a simulada se comportar como um driver que inverte cada trabalho recebido. Sem isso ligado,
/// ela passa tudo adiante como veio.
pub struct DriverDeTeste<B> {
    interno: B,
    inverte: bool,
}

impl<B> DriverDeTeste<B> {
    pub fn novo(interno: B, inverte: bool) -> Self {
        Self { interno, inverte }
    }
}

impl<B: Backend> Backend for DriverDeTeste<B> {
    type Erro = B::Erro;

    fn enviar(&mut self, passada: Passada, paginas: &[Pagina]) -> Result<(), Self::Erro> {
        if !self.inverte {
            return self.interno.enviar(passada, paginas);
        }
        let invertidas: Vec<Pagina> = paginas.iter().rev().copied().collect();
        self.interno.enviar(passada, &invertidas)
    }

    fn consultar(&mut self) -> Result<EstadoEnvio, Self::Erro> {
        self.interno.consultar()
    }

    fn cancelar(&mut self) -> Result<(), Self::Erro> {
        self.interno.cancelar()
    }

    fn maco_de_fora(&mut self, indice: usize, frentes: &[Pagina]) -> Result<(), Self::Erro> {
        if !self.inverte {
            return self.interno.maco_de_fora(indice, frentes);
        }
        let invertidas: Vec<Pagina> = frentes.iter().rev().copied().collect();
        self.interno.maco_de_fora(indice, &invertidas)
    }
}

impl<B: DescreveFalha> DescreveFalha for DriverDeTeste<B> {
    fn motivo(&self) -> Option<String> {
        self.interno.motivo()
    }
    fn trabalho_saiu_da_fila(&self) -> bool {
        self.interno.trabalho_saiu_da_fila()
    }
    fn folhas_conferidas(&self, paginas: &[Pagina]) -> Option<(usize, usize)> {
        self.interno.folhas_conferidas(paginas)
    }
}

#[cfg(test)]
mod testes {
    use super::*;
    use crate::fluxo::{lotes_do_plano, Condutor, Espera, Passo, SimGuiado};
    use crate::job::EstadoJob;
    use crate::Mecanica;
    use plots_core::{
        plano_copias_do_intervalo, plano_lotes_por_papel, DimensoesPagina, ModeloImpressora,
        OrdemPassadas, PerfilImpressora,
    };
    use std::time::Duration;

    const A4: DimensoesPagina = DimensoesPagina::new(210.0, 297.0);
    const A5: DimensoesPagina = DimensoesPagina::new(148.0, 210.0);
    const ESPERA: Espera = Espera {
        intervalo: Duration::from_millis(1),
        consultas: 10,
    };

    fn plano(
        dimensoes: &[DimensoesPagina],
        folhas: usize,
        modelo: ModeloImpressora,
    ) -> Vec<LoteDoFluxo> {
        let lotes =
            plano_lotes_por_papel(dimensoes, folhas, OrdemPassadas::default(), modelo.perfil())
                .unwrap();
        lotes_do_plano(&lotes)
    }

    fn plano_automatico(dimensoes: &[DimensoesPagina]) -> Vec<LoteDoFluxo> {
        let lotes = plots_core::plano_automatico_do_intervalo(
            dimensoes,
            None,
            50,
            OrdemPassadas::default(),
            ModeloImpressora::JatoDeTinta.perfil(),
            plots_core::RepartoAutomatico::PorTrechoDePapel,
        )
        .unwrap();
        lotes_do_plano(&lotes)
    }

    fn numeros(paginas: &[Pagina]) -> Vec<Option<usize>> {
        paginas.iter().map(|pagina| pagina.numero).collect()
    }

    #[test]
    fn lote_de_duas_faces_nao_ganha_marca_e_fica_na_ordem_do_documento() {
        let mut lotes = plano_automatico(&[A4; 5]);
        assert!(lotes[0].lote.duas_faces);
        let antes = lotes[0].lote.passada1.clone();
        marcar_lotes(&mut lotes, IdiomaDaMarca::Portugues);
        compensar_driver(&mut lotes);
        assert!(lotes[0].marcas.is_empty());
        assert_eq!(lotes[0].lote.passada1, antes);
        assert_eq!(
            numeros(&lotes[0].lote.passada1),
            [Some(1), Some(2), Some(3), Some(4), Some(5)],
            "nenhuma folha em branco a mais"
        );
    }

    #[test]
    fn na_mesma_fila_so_o_lote_da_rota_manual_recebe_marca_e_compensacao() {
        let mut lotes = plano_automatico(&[A4, A4, A5, A5]);
        let automatico = lotes[0].lote.passada1.clone();
        let mut manual = lotes[1].lote.passada1.clone();
        manual.reverse();
        marcar_lotes(&mut lotes, IdiomaDaMarca::Ingles);
        compensar_driver(&mut lotes);
        assert_eq!(lotes[0].lote.passada1, automatico);
        assert!(lotes[0].marcas.is_empty());
        assert_eq!(lotes[1].lote.passada1, manual);
        assert_eq!(lotes[1].marcas[&(0, Some(4))], "Sheet 2");
    }

    /// Conduz todos os lotes na simulada e devolve folhas certas e total de folhas.
    fn imprimir(
        lotes: Vec<LoteDoFluxo>,
        modelo: ModeloImpressora,
        inverte: bool,
    ) -> (usize, usize) {
        let simulada = SimGuiado::novo(Mecanica::do_modelo(modelo), modelo.gesto().into());
        let backend = DriverDeTeste::novo(simulada, inverte);
        let mut condutor = Condutor::novo(backend, lotes, ESPERA).unwrap();
        let (mut certas, mut total) = (0, 0);
        condutor.enviar_frente().unwrap();
        loop {
            match condutor.consultar() {
                Passo::FrenteConfirmada => condutor.enviar_verso().unwrap(),
                Passo::LoteConcluido | Passo::DocumentoConcluido => {
                    let paginas = &condutor.lote_terminado().unwrap().lote.paginas;
                    let (a, b) = condutor.backend().folhas_conferidas(paginas).unwrap();
                    certas += a;
                    total += b;
                    if condutor.estado() == EstadoJob::Concluido {
                        return (certas, total);
                    }
                    condutor.enviar_frente().unwrap();
                }
                outro => panic!("passo inesperado: {outro:?}"),
            }
        }
    }

    #[test]
    fn com_a_opcao_o_maco_sai_certo_e_sem_ela_sai_errado_nos_dois_modelos_e_com_total_impar() {
        for modelo in [ModeloImpressora::JatoDeTinta, ModeloImpressora::Laser] {
            for paginas in [8, 9, 5] {
                let dimensoes = vec![A4; paginas];
                let inteiro = plano(&dimensoes, 2, modelo);
                let folhas = paginas.div_ceil(2);

                let referencia = imprimir(inteiro.clone(), modelo, false);
                assert_eq!(
                    referencia,
                    (folhas, folhas),
                    "{modelo:?} sem driver invertendo"
                );

                let (certas, total) = imprimir(inteiro.clone(), modelo, true);
                assert_eq!(total, folhas);
                assert!(
                    certas < total,
                    "{modelo:?} {paginas}: desligada tem de embaralhar"
                );

                let mut compensado = inteiro.clone();
                compensar_driver(&mut compensado);
                assert_eq!(
                    imprimir(compensado, modelo, true),
                    (folhas, folhas),
                    "{modelo:?} {paginas}: ligada tem de sair certa"
                );
            }
        }
    }

    #[test]
    fn papeis_mistos_compensados_saem_certos_e_cada_lote_fica_no_seu_papel() {
        let modelo = ModeloImpressora::Laser;
        let dimensoes = [A4, A4, A4, A4, A5, A5, A5, A5];
        let mut lotes = plano(&dimensoes, 5, modelo);
        assert!(lotes.len() >= 2);
        let papeis: Vec<_> = lotes.iter().map(|item| item.papel).collect();
        compensar_driver(&mut lotes);
        assert_eq!(
            papeis,
            lotes.iter().map(|item| item.papel).collect::<Vec<_>>()
        );
        let (certas, total) = imprimir(lotes, modelo, true);
        assert_eq!(certas, total);
    }

    #[test]
    fn compensar_duas_vezes_volta_ao_plano_e_o_verso_em_branco_acompanha() {
        let inteiro = plano(&[A4; 3], 2, ModeloImpressora::Laser);
        let mut lotes = inteiro.clone();
        compensar_driver(&mut lotes);
        assert!(lotes[0]
            .lote
            .passada1
            .iter()
            .chain(&lotes[0].lote.passada2)
            .any(|face| face.numero.is_none()));
        assert_ne!(lotes, inteiro);
        compensar_driver(&mut lotes);
        assert_eq!(lotes, inteiro);
    }

    #[test]
    fn a_marca_so_entra_no_verso_e_nao_muda_a_ordem_nem_a_quantidade() {
        let inteiro = plano(&[A4; 7], 2, ModeloImpressora::JatoDeTinta);
        let mut marcados = inteiro.clone();
        marcar_lotes(&mut marcados, IdiomaDaMarca::Portugues);
        for (antes, depois) in inteiro.iter().zip(&marcados) {
            assert_eq!(antes.lote, depois.lote);
            assert_eq!(depois.marcas.len(), depois.lote.paginas.len().div_ceil(2));
            for numero in depois
                .marcas
                .keys()
                .filter_map(|(_, numero)| numero.as_ref())
            {
                assert_eq!(numero % 2, 0, "a página {numero} é a frente da folha");
            }
        }
        assert!(inteiro.iter().all(|item| item.marcas.is_empty()));
    }

    #[test]
    fn na_laser_a_marca_sai_na_passada_sem_giro_e_a_passada_girada_fica_sem_marca() {
        let mut lotes = plano(&[A4; 6], 3, ModeloImpressora::Laser);
        marcar_lotes(&mut lotes, IdiomaDaMarca::Portugues);
        let lote = &lotes[0];
        assert!(lote.lote.passada1.iter().all(|pagina| !pagina.girar_180));
        assert!(lote.lote.passada2.iter().all(|pagina| pagina.girar_180));
        assert!(lote
            .lote
            .passada1
            .iter()
            .all(|pagina| lote.marcas.contains_key(&pagina.identidade())));
        assert!(lote
            .lote
            .passada2
            .iter()
            .all(|pagina| !lote.marcas.contains_key(&pagina.identidade())));
    }

    #[test]
    fn a_marca_conta_a_folha_no_documento_e_acompanha_o_verso_em_branco() {
        let mut lotes = plano(&[A4; 7], 2, ModeloImpressora::Laser);
        marcar_lotes(&mut lotes, IdiomaDaMarca::Portugues);
        assert_eq!(lotes[0].marcas[&(0, Some(2))], "Folha 1");
        assert_eq!(lotes[0].marcas[&(0, Some(4))], "Folha 2");
        assert_eq!(lotes[1].marcas[&(0, Some(6))], "Folha 3");
        assert_eq!(lotes[1].marcas[&(0, None)], "Folha 4");
        marcar_lotes(&mut lotes, IdiomaDaMarca::Ingles);
        assert_eq!(lotes[1].marcas[&(0, None)], "Sheet 4");
    }

    /// Os textos de marca na ordem em que as páginas são enviadas: passada 1 e depois passada 2, lote a lote.
    fn marcas_enviadas(lotes: &[LoteDoFluxo]) -> Vec<(usize, String)> {
        let numero = |texto: &String| texto.trim_start_matches("Folha ").parse().unwrap();
        lotes
            .iter()
            .flat_map(|item| {
                item.lote
                    .passada1
                    .iter()
                    .chain(&item.lote.passada2)
                    .filter_map(|pagina| item.marcas.get(&pagina.identidade()))
            })
            .map(|texto| (numero(texto), texto.clone()))
            .collect()
    }

    fn com_copias(
        paginas: usize,
        copias: usize,
        perfil: PerfilImpressora,
        compensado: bool,
    ) -> Vec<LoteDoFluxo> {
        let plano = plano_copias_do_intervalo(
            &vec![A4; paginas],
            None,
            copias,
            OrdemPassadas::default(),
            perfil,
        )
        .unwrap();
        let mut lotes = lotes_do_plano(&plano);
        marcar_lotes(&mut lotes, IdiomaDaMarca::Portugues);
        if compensado {
            compensar_driver(&mut lotes);
        }
        lotes
    }

    fn perfis_de_teste() -> [PerfilImpressora; 3] {
        let sem_inverter = PerfilImpressora {
            passada1_inversa: false,
            passada2_inversa: false,
            verso_girar_180: false,
            ..ModeloImpressora::Laser.perfil()
        };
        [
            ModeloImpressora::JatoDeTinta.perfil(),
            ModeloImpressora::Laser.perfil(),
            sem_inverter,
        ]
    }

    #[test]
    fn com_copias_cada_folha_tem_a_sua_marca_unica_e_continua() {
        for perfil in perfis_de_teste() {
            for paginas in [24, 7] {
                for copias in [1, 3] {
                    for compensado in [false, true] {
                        let lotes = com_copias(paginas, copias, perfil, compensado);
                        let mut numeros: Vec<usize> = marcas_enviadas(&lotes)
                            .into_iter()
                            .map(|(numero, _)| numero)
                            .collect();
                        numeros.sort_unstable();
                        let folhas = copias * paginas.div_ceil(2);
                        assert_eq!(
                            numeros,
                            (1..=folhas).collect::<Vec<_>>(),
                            "{paginas} páginas, {copias} cópias, compensado {compensado}, {perfil:?}"
                        );
                    }
                }
            }
        }
    }

    #[test]
    fn com_tres_copias_de_24_paginas_as_marcas_vao_de_folha_1_a_folha_36() {
        let lotes = com_copias(24, 3, ModeloImpressora::Laser.perfil(), false);
        let mut textos: Vec<String> = marcas_enviadas(&lotes)
            .into_iter()
            .map(|(_, texto)| texto)
            .collect();
        textos.sort_by_key(|texto| texto.trim_start_matches("Folha ").parse::<usize>().unwrap());
        let esperado: Vec<String> = (1..=36).map(|n| format!("Folha {n}")).collect();
        assert_eq!(textos, esperado);
    }

    #[test]
    fn a_ordem_das_marcas_na_pilha_nao_muda_com_copias_nem_com_o_driver_que_inverte() {
        for perfil in perfis_de_teste() {
            for paginas in [24, 7] {
                for copias in [1, 3] {
                    let sem_driver = com_copias(paginas, copias, perfil, false);
                    let esperado = marcas_enviadas(&sem_driver);
                    // O driver inverte cada trabalho: com a compensação ligada o que chega à
                    // impressora é a mesma ordem de quem não tem driver que inverte.
                    let compensado = com_copias(paginas, copias, perfil, true);
                    let mut recebido = Vec::new();
                    for item in &compensado {
                        let mut passada1: Vec<_> = item.lote.passada1.iter().rev().collect();
                        let passada2: Vec<_> = item.lote.passada2.iter().rev().collect();
                        passada1.extend(passada2);
                        recebido.extend(
                            passada1
                                .iter()
                                .filter_map(|pagina| item.marcas.get(&pagina.identidade()))
                                .cloned(),
                        );
                    }
                    let esperado: Vec<String> =
                        esperado.into_iter().map(|(_, texto)| texto).collect();
                    assert_eq!(recebido, esperado, "{paginas} páginas, {copias} cópias");
                }
            }
        }
    }

    #[test]
    fn com_uma_copia_as_marcas_ficam_como_sempre() {
        let lotes = com_copias(7, 1, ModeloImpressora::Laser.perfil(), false);
        let item = &lotes[0];
        assert_eq!(item.marcas[&(0, Some(2))], "Folha 1");
        assert_eq!(item.marcas[&(0, Some(6))], "Folha 3");
        assert_eq!(item.marcas[&(0, None)], "Folha 4");
        assert_eq!(item.marcas.len(), 4);
    }

    #[test]
    fn a_marca_segue_a_pagina_mesmo_com_a_passada_na_ordem_contraria() {
        let mut lotes = plano(&[A4; 4], 2, ModeloImpressora::Laser);
        marcar_lotes(&mut lotes, IdiomaDaMarca::Portugues);
        let marcas = lotes[0].marcas.clone();
        compensar_driver(&mut lotes);
        assert_eq!(lotes[0].marcas, marcas);
        assert_eq!(lotes[0].marcas[&(0, Some(2))], "Folha 1");
    }
}
