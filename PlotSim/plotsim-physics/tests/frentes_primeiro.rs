//! Todas as frentes em um trabalho, depois todos os versos, conferidos pelo modelo físico: o maço
//! inteiro, e também quando ele não cabe de uma vez na entrada.

mod comum;

use comum::*;
use plots_core::{
    plano_copias_do_intervalo, plano_em_trechos_do_intervalo, plano_lotes_do_intervalo,
    DimensoesPagina, LotePapel, ModeloImpressora, OrdemPassadas, Pagina,
};
use plots_print::fluxo::{lotes_do_plano, Condutor, DescreveFalha, Espera, Passo};
use plots_print::job::{Alcance, Backend, EstadoEnvio, EstadoJob, Passada};
use plotsim_physics::{conferir, ler_maco, Envio, Gesto, Impressora, MacoLido, Motivo, Pilha};
use std::time::Duration;

const A4: DimensoesPagina = DimensoesPagina::new(210.0, 297.0);
const A5: DimensoesPagina = DimensoesPagina::new(148.0, 210.0);
const MODELOS: [ModeloImpressora; 2] = [ModeloImpressora::JatoDeTinta, ModeloImpressora::Laser];
const SEM_DEMORA: Espera = Espera {
    intervalo: Duration::ZERO,
    consultas: 10,
};

fn trechos(dimensoes: &[DimensoesPagina], modelo: ModeloImpressora) -> Vec<LotePapel> {
    plano_em_trechos_do_intervalo(dimensoes, None, OrdemPassadas::default(), modelo.perfil())
        .expect("o plano aceita o documento")
}

/// Liga o programa conferido ao modelo físico: cada envio vira uma passada na máquina, e o verso
/// recoloca o maço com o gesto que o programa dá ao modelo.
struct Fisica {
    modelo: ModeloImpressora,
    impressora: Impressora,
    gesto: Gesto,
    passadas: Vec<Passada>,
    macos: Vec<MacoLido>,
}

impl Fisica {
    fn nova(modelo: ModeloImpressora) -> Self {
        let perfil = modelo.perfil();
        Self {
            modelo,
            impressora: Impressora::nova(modo_do_modelo(modelo), 0),
            gesto: gesto_do_modelo(modelo, borda_que_fecha(perfil)),
            passadas: Vec::new(),
            macos: Vec::new(),
        }
    }

    fn guardar_o_que_saiu(&mut self) {
        if !self.impressora.saida().is_empty() {
            let leitura = leitura_do_modelo(self.modelo);
            self.macos.push(ler_maco(self.impressora.saida(), leitura));
            self.impressora.retirar_saida();
        }
    }
}

impl Backend for Fisica {
    type Erro = String;
    fn enviar(&mut self, passada: Passada, paginas: &[Pagina]) -> Result<(), String> {
        self.passadas.push(passada);
        match passada {
            Passada::Frente => {
                self.guardar_o_que_saiu();
                self.impressora.abastecer(paginas.len());
            }
            Passada::Verso => {
                self.impressora
                    .recolocar(self.gesto)
                    .map_err(|erro| format!("{erro:?}"))?;
            }
            Passada::DuasFaces => return Err("este modo não usa duas faces".to_string()),
        }
        let resumo = self.impressora.imprimir(&envios(paginas));
        if resumo.sem_folha.is_empty() {
            Ok(())
        } else {
            Err("faltou folha na entrada".to_string())
        }
    }
    fn consultar(&mut self) -> Result<EstadoEnvio, String> {
        Ok(EstadoEnvio::Concluido)
    }
    fn cancelar(&mut self) -> Result<(), String> {
        Ok(())
    }
}

impl DescreveFalha for Fisica {
    fn motivo(&self) -> Option<String> {
        None
    }
    fn trabalho_saiu_da_fila(&self) -> bool {
        false
    }
}

/// Conduz o documento clicando só no que o cartão oferece e devolve as passadas e os maços.
fn conduzir(modelo: ModeloImpressora, lotes: &[LotePapel]) -> (Vec<Passada>, Vec<MacoLido>) {
    let mut condutor =
        Condutor::novo(Fisica::nova(modelo), lotes_do_plano(lotes), SEM_DEMORA).expect("há lotes");
    condutor.enviar_frente().expect("frente");
    loop {
        match condutor.consultar() {
            Passo::FrenteConfirmada => condutor.enviar_verso().expect("verso"),
            Passo::LoteConcluido => condutor.enviar_frente().expect("próximo trecho"),
            Passo::DocumentoConcluido => break,
            outro => panic!("passo inesperado: {outro:?}"),
        }
    }
    assert_eq!(condutor.estado(), EstadoJob::Concluido);
    let mut fisica = condutor;
    let passadas = fisica.backend().passadas.clone();
    let mut macos = fisica.backend().macos.clone();
    let leitura = leitura_do_modelo(modelo);
    macos.push(ler_maco(fisica.backend().impressora.saida(), leitura));
    let _ = &mut fisica;
    (passadas, macos)
}

#[test]
fn vinte_e_quatro_paginas_com_dez_no_campo_dao_um_trabalho_de_frentes_e_um_de_versos() {
    for modelo in MODELOS {
        let documento = [A4; 24];
        let com_o_campo = plano_lotes_do_intervalo(
            &documento,
            None,
            10,
            OrdemPassadas::default(),
            modelo.perfil(),
        )
        .expect("o plano aceita o documento");
        assert_eq!(com_o_campo.len(), 2, "lote a lote para a cada dez folhas");

        let plano = trechos(&documento, modelo);
        assert_eq!(plano.len(), 1);
        assert_eq!(plano[0].passada1.len(), 12);
        let (passadas, macos) = conduzir(modelo, &plano);
        assert_eq!(passadas, [Passada::Frente, Passada::Verso], "{modelo:?}");
        assert_eq!(
            conferir(&juntar(macos), 24),
            Vec::<Motivo>::new(),
            "{modelo:?}"
        );
    }
}

#[test]
fn qualquer_total_de_paginas_fecha_nos_dois_modelos_com_uma_frente_e_um_verso() {
    for modelo in MODELOS {
        for total in [1, 2, 3, 9, 23, 24, 101] {
            let plano = trechos(&vec![A4; total], modelo);
            assert_eq!(plano.len(), 1);
            let (passadas, macos) = conduzir(modelo, &plano);
            assert_eq!(passadas.len(), 2);
            let total = u32::try_from(total).expect("total cabe em u32");
            assert_eq!(
                conferir(&juntar(macos), total),
                Vec::<Motivo>::new(),
                "{modelo:?} com {total} páginas",
            );
        }
    }
}

#[test]
fn cada_trecho_de_mesmo_papel_e_um_trabalho_e_os_macos_juntos_formam_o_documento() {
    for modelo in MODELOS {
        let mut documento = vec![A4; 8];
        documento.extend([A5; 6]);
        documento.extend([A4; 10]);
        let plano = trechos(&documento, modelo);
        assert_eq!(plano.len(), 3);
        let (passadas, macos) = conduzir(modelo, &plano);
        assert_eq!(
            passadas,
            [
                Passada::Frente,
                Passada::Verso,
                Passada::Frente,
                Passada::Verso,
                Passada::Frente,
                Passada::Verso,
            ]
        );
        assert_eq!(
            conferir(&juntar(macos), 24),
            Vec::<Motivo>::new(),
            "{modelo:?}"
        );
    }
}

/// As primeiras `quantas` folhas de cima da pilha, sem tocar nela.
fn parte_de_cima(pilha: &Pilha, quantas: usize) -> Pilha {
    let mut parte = pilha.clone();
    while parte.len() > quantas {
        parte.retirar_de_cima(quantas);
    }
    parte
}

/// O que sobra embaixo depois de tirar as primeiras `quantas` folhas de cima.
fn parte_de_baixo(pilha: &Pilha, quantas: usize) -> Pilha {
    let mut parte = pilha.clone();
    for _ in 0..quantas {
        parte.retirar_de_cima(0);
    }
    parte
}

/// Põe uma pilha na entrada vazia sem mexer no que já saiu nem na ordem das folhas.
fn colocar_na_entrada(impressora: &mut Impressora, pilha: Pilha) {
    let ja_saiu = impressora.retirar_saida();
    *impressora.saida_mut() = pilha;
    impressora.recolocar_com(&[]);
    *impressora.saida_mut() = ja_saiu;
}

/// A entrada só cabe `capacidade` folhas. A frente é abastecida aos poucos, e o verso recebe o maço
/// em partes: `primeiro` diz qual parte a pessoa coloca antes.
fn imprimir_em_partes(
    modelo: ModeloImpressora,
    total: usize,
    capacidade: usize,
    primeiro: fn(&Pilha, usize) -> (Pilha, Pilha),
) -> Vec<Motivo> {
    let plano = trechos(&vec![A4; total], modelo);
    let lote = &plano[0];
    let mut impressora = Impressora::nova(modo_do_modelo(modelo), 0);
    for parte in envios(&lote.passada1).chunks(capacidade) {
        impressora.abastecer(parte.len());
        assert!(impressora.imprimir(parte).sem_folha.is_empty());
    }
    let gesto = gesto_do_modelo(modelo, borda_que_fecha(modelo.perfil()));
    let mut maco = impressora.retirar_saida();
    let operacoes = gesto.operacoes(&maco).expect("o gesto cabe na máquina");
    maco.aplicar_todas(&operacoes);

    let versos: Vec<Envio> = envios(&lote.passada2);
    let mut restantes = versos.as_slice();
    let mut resto = maco;
    while !resto.is_empty() {
        let quantas = capacidade.min(resto.len());
        let (agora, depois) = primeiro(&resto, quantas);
        let (nesta, seguintes) = restantes.split_at(quantas);
        colocar_na_entrada(&mut impressora, agora);
        assert!(impressora.imprimir(nesta).sem_folha.is_empty());
        restantes = seguintes;
        resto = depois;
    }
    let leitura = leitura_do_modelo(modelo);
    let lido = ler_maco(impressora.saida(), leitura);
    conferir(&lido, u32::try_from(total).expect("total cabe em u32"))
}

#[test]
fn maco_maior_que_a_entrada_sai_certo_colocando_a_parte_de_cima_e_depois_o_resto() {
    for modelo in MODELOS {
        for total in [24usize, 23] {
            for capacidade in 1..=total.div_ceil(2) {
                let motivos = imprimir_em_partes(modelo, total, capacidade, |pilha, quantas| {
                    (
                        parte_de_cima(pilha, quantas),
                        parte_de_baixo(pilha, quantas),
                    )
                });
                assert_eq!(
                    motivos,
                    Vec::<Motivo>::new(),
                    "{modelo:?}, {total} páginas, entrada para {capacidade}",
                );
            }
        }
    }
}

#[test]
fn colocar_antes_a_parte_de_baixo_erra_o_maco_nos_dois_modelos() {
    for modelo in MODELOS {
        let motivos = imprimir_em_partes(modelo, 24, 5, |pilha, quantas| {
            let debaixo = pilha.len() - quantas;
            (
                parte_de_baixo(pilha, debaixo),
                parte_de_cima(pilha, debaixo),
            )
        });
        assert_ne!(motivos, Vec::<Motivo>::new(), "{modelo:?}");
    }
}

/// Só as frentes: cada trecho deixa o seu maço, e o programa não envia verso nenhum.
struct SoFrentes {
    impressora: Impressora,
    maços: Vec<Pilha>,
    passadas: Vec<Passada>,
}

impl Backend for SoFrentes {
    type Erro = String;
    fn enviar(&mut self, passada: Passada, paginas: &[Pagina]) -> Result<(), String> {
        self.passadas.push(passada);
        if passada != Passada::Frente {
            return Err("este trabalho imprime só frentes".to_string());
        }
        if !self.impressora.saida().is_empty() {
            self.maços.push(self.impressora.retirar_saida());
        }
        self.impressora.abastecer(paginas.len());
        let resumo = self.impressora.imprimir(&envios(paginas));
        if resumo.sem_folha.is_empty() {
            Ok(())
        } else {
            Err("faltou folha na entrada".to_string())
        }
    }
    fn consultar(&mut self) -> Result<EstadoEnvio, String> {
        Ok(EstadoEnvio::Concluido)
    }
    fn cancelar(&mut self) -> Result<(), String> {
        Ok(())
    }
}

/// Só os versos: cada maço volta à entrada com o gesto do modelo, na hora do seu trecho.
struct SoVersos {
    modelo: ModeloImpressora,
    impressora: Impressora,
    gesto: Gesto,
    maços_de_frente: Vec<Pilha>,
    macos: Vec<MacoLido>,
    passadas: Vec<Passada>,
}

impl Backend for SoVersos {
    type Erro = String;
    fn enviar(&mut self, passada: Passada, paginas: &[Pagina]) -> Result<(), String> {
        self.passadas.push(passada);
        if passada != Passada::Verso {
            return Err("este trabalho imprime só versos".to_string());
        }
        let resumo = self.impressora.imprimir(&envios(paginas));
        if !resumo.sem_folha.is_empty() {
            return Err("faltou folha na entrada".to_string());
        }
        let saiu = self.impressora.retirar_saida();
        self.macos
            .push(ler_maco(&saiu, leitura_do_modelo(self.modelo)));
        Ok(())
    }
    fn consultar(&mut self) -> Result<EstadoEnvio, String> {
        Ok(EstadoEnvio::Concluido)
    }
    fn cancelar(&mut self) -> Result<(), String> {
        Ok(())
    }
    fn maco_de_fora(&mut self, indice: usize, _frentes: &[Pagina]) -> Result<(), String> {
        let mut maco = self
            .maços_de_frente
            .get(indice)
            .cloned()
            .ok_or_else(|| "não há maço de frente para este trecho".to_string())?;
        let operacoes = self
            .gesto
            .operacoes(&maco)
            .map_err(|erro| format!("{erro:?}"))?;
        maco.aplicar_todas(&operacoes);
        colocar_na_entrada(&mut self.impressora, maco);
        Ok(())
    }
}

/// Todas as frentes em um trabalho só, e depois cada maço para o seu verso.
fn frentes_depois_versos(
    modelo: ModeloImpressora,
    lotes: &[LotePapel],
) -> (Vec<Pilha>, Vec<Passada>, Vec<MacoLido>, Vec<Passada>) {
    let mut frentes = Condutor::novo(
        SoFrentes {
            impressora: Impressora::nova(modo_do_modelo(modelo), 0),
            maços: Vec::new(),
            passadas: Vec::new(),
        },
        lotes_do_plano(lotes),
        SEM_DEMORA,
    )
    .expect("há lotes")
    .com_alcance(Alcance::SoFrentes);
    frentes.enviar_frente().expect("primeira frente");
    loop {
        match frentes.consultar() {
            Passo::LoteConcluido => frentes.enviar_frente().expect("frente do próximo trecho"),
            Passo::DocumentoConcluido => break,
            outro => panic!("passo inesperado nas frentes: {outro:?}"),
        }
    }
    assert_eq!(frentes.estado(), EstadoJob::Concluido);
    let feitas = frentes.backend();
    let mut maços = feitas.maços.clone();
    if !feitas.impressora.saida().is_empty() {
        maços.push(feitas.impressora.saida().clone());
    }
    let passadas_de_frente = feitas.passadas.clone();

    let mut versos = Condutor::novo(
        SoVersos {
            modelo,
            impressora: Impressora::nova(modo_do_modelo(modelo), 0),
            gesto: gesto_do_modelo(modelo, borda_que_fecha(modelo.perfil())),
            maços_de_frente: maços.clone(),
            macos: Vec::new(),
            passadas: Vec::new(),
        },
        lotes_do_plano(lotes),
        SEM_DEMORA,
    )
    .expect("há lotes")
    .com_alcance(Alcance::SoVersos);
    versos.enviar_verso().expect("verso do primeiro trecho");
    loop {
        match versos.consultar() {
            Passo::LoteConcluido => versos.enviar_verso().expect("verso do próximo trecho"),
            Passo::DocumentoConcluido => break,
            outro => panic!("passo inesperado nos versos: {outro:?}"),
        }
    }
    assert_eq!(versos.estado(), EstadoJob::Concluido);
    let depois = versos.backend();
    (
        maços,
        passadas_de_frente,
        depois.macos.clone(),
        depois.passadas.clone(),
    )
}

#[test]
fn frentes_de_todos_os_trechos_saem_antes_e_cada_maco_volta_para_o_seu_verso() {
    let mista = {
        let mut documento = vec![A4; 8];
        documento.extend([A5; 6]);
        documento.extend([A4; 10]);
        documento
    };
    for modelo in MODELOS {
        for documento in [mista.clone(), vec![A4; 23]] {
            let total = u32::try_from(documento.len()).expect("total cabe em u32");
            let plano = trechos(&documento, modelo);
            let (maços, de_frente, macos, de_verso) = frentes_depois_versos(modelo, &plano);
            assert_eq!(maços.len(), plano.len(), "{modelo:?}, {total} páginas");
            assert_eq!(de_frente, vec![Passada::Frente; plano.len()]);
            assert_eq!(de_verso, vec![Passada::Verso; plano.len()]);
            assert_eq!(
                conferir(&juntar(macos), total),
                Vec::<Motivo>::new(),
                "{modelo:?}, {total} páginas",
            );
        }
    }
}

impl DescreveFalha for SoFrentes {
    fn motivo(&self) -> Option<String> {
        None
    }
    fn trabalho_saiu_da_fila(&self) -> bool {
        false
    }
}

impl DescreveFalha for SoVersos {
    fn motivo(&self) -> Option<String> {
        None
    }
    fn trabalho_saiu_da_fila(&self) -> bool {
        false
    }
}

fn copias_do_documento(
    documento: &[DimensoesPagina],
    copias: usize,
    modelo: ModeloImpressora,
) -> Vec<LotePapel> {
    plano_copias_do_intervalo(
        documento,
        None,
        copias,
        OrdemPassadas::default(),
        modelo.perfil(),
    )
    .expect("o plano aceita as cópias")
}

/// Cada folha do maço, na ordem do documento: a página da frente e a do verso, ou nenhuma no branco.
fn folhas_esperadas(lotes: &[LotePapel]) -> Vec<(Vec<u32>, Vec<u32>)> {
    let para_u32 = |numero: Option<usize>| -> Vec<u32> {
        numero
            .map(|n| vec![u32::try_from(n).expect("página cabe em u32")])
            .unwrap_or_default()
    };
    lotes
        .iter()
        .flat_map(|lote| {
            lote.paginas
                .chunks(2)
                .map(move |par| {
                    (
                        para_u32(par.first().copied().flatten()),
                        para_u32(par.get(1).copied().flatten()),
                    )
                })
                .collect::<Vec<_>>()
        })
        .collect()
}

/// Compara o maço lido, folha por folha, com o que o plano de cópias pede.
fn confere_copias(maco: &MacoLido, lotes: &[LotePapel]) -> Vec<String> {
    let esperadas = folhas_esperadas(lotes);
    let mut erros = Vec::new();
    if maco.folhas.len() != esperadas.len() {
        erros.push(format!(
            "{} folhas, eram {}",
            maco.folhas.len(),
            esperadas.len()
        ));
    }
    for (indice, (folha, (frente, verso))) in maco.folhas.iter().zip(&esperadas).enumerate() {
        if &folha.frente != frente || &folha.verso != verso {
            erros.push(format!(
                "folha {}: frente {:?} verso {:?}, era {:?} / {:?}",
                indice + 1,
                folha.frente,
                folha.verso,
                frente,
                verso
            ));
        }
    }
    erros
}

#[test]
fn copias_saem_no_maco_certo_com_total_par_e_impar_nos_dois_modelos() {
    for modelo in MODELOS {
        for (paginas, copias) in [(4usize, 2usize), (3, 2), (3, 3), (4, 3), (1, 3)] {
            let documento = vec![A4; paginas];
            let lotes = copias_do_documento(&documento, copias, modelo);
            let (_, _, macos, _) = frentes_depois_versos(modelo, &lotes);
            assert_eq!(
                confere_copias(&juntar(macos), &lotes),
                Vec::<String>::new(),
                "{modelo:?}, {copias} cópias de {paginas} páginas",
            );
        }
    }
}

#[test]
fn toda_pagina_sai_exatamente_uma_vez_por_copia_e_o_branco_nao_conta() {
    for modelo in MODELOS {
        for (paginas, copias) in [(5usize, 3usize), (6, 2)] {
            let documento = vec![A4; paginas];
            let lotes = copias_do_documento(&documento, copias, modelo);
            let (_, _, macos, _) = frentes_depois_versos(modelo, &lotes);
            let maco = juntar(macos);
            let mut vezes = vec![0usize; paginas + 1];
            for folha in &maco.folhas {
                for numero in folha.frente.iter().chain(folha.verso.iter()) {
                    let indice = usize::try_from(*numero).expect("número cabe em usize");
                    if let Some(contador) = vezes.get_mut(indice) {
                        *contador += 1;
                    }
                }
            }
            for numero in 1..=paginas {
                assert_eq!(
                    vezes[numero], copias,
                    "{modelo:?}, página {numero}, {copias} cópias"
                );
            }
        }
    }
}

#[test]
fn o_conferidor_reprova_um_maco_com_uma_copia_a_menos() {
    for modelo in MODELOS {
        let documento = vec![A4; 3];
        let uma = copias_do_documento(&documento, 1, modelo);
        let duas = copias_do_documento(&documento, 2, modelo);
        let (_, _, macos, _) = frentes_depois_versos(modelo, &uma);
        assert_ne!(
            confere_copias(&juntar(macos), &duas),
            Vec::<String>::new(),
            "{modelo:?}"
        );
    }
}
