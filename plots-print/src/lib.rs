//! Modelo físico simplificado para ensaiar a recolocação de um lote.
//!
//! As pilhas são representadas de baixo para cima. A mecânica é independente
//! do perfil que gerou a ordem das páginas; deve ser calibrada para cada máquina.

use plots_core::{GestoRecolocar, ModeloImpressora, Pagina, ViradaMaco};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct Marca {
    pub pagina: Option<usize>,
    pub girada: bool,
}

/// Faces A e B identificam os dois lados físicos da folha. Uma marca com
/// `pagina: None` representa uma página vazia enviada à impressora.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Folha {
    pub face_a: Option<Marca>,
    pub face_b: Option<Marca>,
    pub face_a_para_cima: bool,
    pub girada: bool,
    /// Segunda impressão sobre uma face já marcada: (face A, nova marca).
    pub sobreimpressao: Option<(bool, Marca)>,
}

/// Comportamento da máquina, medido independentemente do plano de envio.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct Mecanica {
    /// A primeira face impressa sai para cima.
    pub saida_face_impressa_para_cima: bool,
    /// Cada folha sai em cima da pilha existente; caso contrário, sai embaixo.
    pub saida_no_topo: bool,
    /// Face que recebe tinta quando a folha é puxada da entrada.
    pub imprime_face_para_cima: bool,
    /// A entrada retira a folha de cima; caso contrário, retira a de baixo.
    pub entrada_pelo_topo: bool,
    /// A máquina gira o conteúdo da segunda passagem relativamente à folha.
    pub giro_segunda_passada: bool,
}

impl Mecanica {
    /// Jato de tinta: a folha sai com a face impressa para cima, cai em cima da
    /// pilha, e a entrada puxa a folha de cima, que recebe tinta na face de cima.
    pub const fn traseira() -> Self {
        Self {
            saida_face_impressa_para_cima: true,
            saida_no_topo: true,
            imprime_face_para_cima: true,
            entrada_pelo_topo: true,
            giro_segunda_passada: false,
        }
    }

    /// Laser: a folha sai com a face impressa para baixo, cai em cima da pilha, e a
    /// gaveta puxa a folha de cima. A gaveta imprime a face que fica para cima, por
    /// isso o maço não é virado na recolocação. A curva em U troca a aresta que lidera:
    /// o verso vai girado. O maço final sai com a primeira folha por baixo e é lido
    /// virando-o.
    pub const fn gaveta() -> Self {
        Self {
            saida_face_impressa_para_cima: false,
            saida_no_topo: true,
            imprime_face_para_cima: true,
            entrada_pelo_topo: true,
            giro_segunda_passada: false,
        }
    }

    /// A folha faz uma curva em U quando entra com a face a imprimir para um lado e sai com ela
    /// para o outro: a face de cima passa para baixo, e a aresta que entrou primeiro passa a ser a
    /// última a entrar de novo. No caminho reto, a folha só anda.
    pub const fn curva_em_u(self) -> bool {
        self.imprime_face_para_cima != self.saida_face_impressa_para_cima
    }

    /// Mecânica do tipo de máquina que o modelo representa.
    pub const fn do_modelo(modelo: ModeloImpressora) -> Self {
        match modelo {
            ModeloImpressora::JatoDeTinta => Self::traseira(),
            ModeloImpressora::Laser => Self::gaveta(),
        }
    }
}

/// Operações feitas no maço antes de devolvê-lo à entrada. Virar um maço
/// também inverte a ordem das folhas; `inverter_ordem` é uma operação adicional.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct Recolocacao {
    pub virar_maco: bool,
    pub inverter_ordem: bool,
    pub girar_180: bool,
    /// Quantidade de folhas retiradas do topo, para simular uma perda.
    pub retirar_folhas: usize,
}

impl Recolocacao {
    /// Virada pela borda de cima, como um bloco de notas. O maço vira como em
    /// `virar_maco`, e cada folha também gira 180° no próprio plano: a borda que
    /// estava embaixo passa para o lado de quem lê. Por isso a folha fica de cabeça
    /// para baixo em relação à frente, a não ser que o verso seja impresso girado.
    pub const fn virada_pela_borda_de_cima() -> Self {
        Self {
            virar_maco: true,
            inverter_ordem: false,
            girar_180: true,
            retirar_folhas: 0,
        }
    }
}

impl From<GestoRecolocar> for Recolocacao {
    /// A borda que entra primeiro não muda a ordem nem a face das folhas no simulador.
    fn from(gesto: GestoRecolocar) -> Self {
        match gesto.virada {
            ViradaMaco::NaoVirar => Self::default(),
            ViradaMaco::PelaLateral => Self {
                virar_maco: true,
                ..Self::default()
            },
            ViradaMaco::PelaBordaDeCima => Self::virada_pela_borda_de_cima(),
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Estado {
    Ocioso,
    AguardandoRecolocacao,
    ProntoParaVerso,
    Concluido,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum ErroSimulacao {
    EstadoInvalido {
        esperado: Estado,
        atual: Estado,
    },
    QuantidadeFolhas {
        esperadas: usize,
        disponiveis: usize,
    },
    /// A máquina não imprime as duas faces deste pedido: o papel não é aceito, ou ela não tem o
    /// recurso. Nada sai.
    DuasFacesRecusadas,
}

impl std::fmt::Display for ErroSimulacao {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::EstadoInvalido { esperado, atual } => {
                write!(f, "estado inválido: esperado {esperado:?}, atual {atual:?}")
            }
            Self::QuantidadeFolhas {
                esperadas,
                disponiveis,
            } => write!(
                f,
                "quantidade de folhas: necessárias {esperadas}, disponíveis {disponiveis}"
            ),
            Self::DuasFacesRecusadas => {
                f.write_str("a impressora não aceitou imprimir nas duas faces; nada foi enviado")
            }
        }
    }
}
impl std::error::Error for ErroSimulacao {}

pub struct Simulador {
    mecanica: Mecanica,
    estado: Estado,
    pilha: Vec<Folha>,
}

impl Simulador {
    pub fn novo(mecanica: Mecanica) -> Self {
        Self {
            mecanica,
            estado: Estado::Ocioso,
            pilha: Vec::new(),
        }
    }

    pub fn estado(&self) -> Estado {
        self.estado
    }
    /// Pilha na saída ou na entrada, em ordem de baixo para cima.
    pub fn pilha(&self) -> &[Folha] {
        &self.pilha
    }

    /// Folhas do lote concluído na ordem em que o maço é lido, de baixo para
    /// cima (a última da lista é a primeira a ser lida).
    ///
    /// Com a face impressa para cima, o maço é lido como está na saída. Com a
    /// face impressa para baixo, o maço sai com a última folha por cima e é
    /// lido depois de virado, o que inverte a ordem: a folha 1|2 fica por cima.
    pub fn folhas_para_leitura(&self) -> Vec<Folha> {
        let mut folhas = self.pilha.clone();
        if !self.mecanica.saida_face_impressa_para_cima {
            folhas.reverse();
        }
        folhas
    }

    /// Libera a máquina para outro lote e entrega a pilha concluída.
    pub fn retirar_lote(&mut self) -> Result<Vec<Folha>, ErroSimulacao> {
        self.exige(Estado::Concluido)?;
        self.estado = Estado::Ocioso;
        Ok(std::mem::take(&mut self.pilha))
    }

    /// Cancela o lote ainda em andamento e devolve as folhas para inspeção.
    /// O lote descartado pode então ser iniciado novamente desde a frente.
    pub fn cancelar_lote(&mut self) -> Result<Vec<Folha>, ErroSimulacao> {
        match self.estado {
            Estado::AguardandoRecolocacao | Estado::ProntoParaVerso => {
                self.estado = Estado::Ocioso;
                Ok(std::mem::take(&mut self.pilha))
            }
            atual => Err(ErroSimulacao::EstadoInvalido {
                esperado: Estado::ProntoParaVerso,
                atual,
            }),
        }
    }

    fn exige(&self, esperado: Estado) -> Result<(), ErroSimulacao> {
        if self.estado == esperado {
            Ok(())
        } else {
            Err(ErroSimulacao::EstadoInvalido {
                esperado,
                atual: self.estado,
            })
        }
    }

    pub fn imprimir_frente(&mut self, paginas: &[Pagina]) -> Result<(), ErroSimulacao> {
        self.exige(Estado::Ocioso)?;
        for pagina in paginas {
            let folha = Folha {
                face_a: Some(Marca {
                    pagina: pagina.numero,
                    girada: pagina.girar_180,
                }),
                face_b: None,
                face_a_para_cima: self.mecanica.saida_face_impressa_para_cima,
                girada: self.mecanica.curva_em_u(),
                sobreimpressao: None,
            };
            self.pilha.push(folha);
        }
        if !self.mecanica.saida_no_topo {
            self.pilha.reverse();
        }
        self.estado = Estado::AguardandoRecolocacao;
        Ok(())
    }

    /// A máquina imprime as duas faces de cada folha sozinha, nesta ordem: cada duas páginas formam
    /// uma folha, e a primeira folha fica por baixo. Quem envia não manda face em branco nem página
    /// girada: um total ímpar deixa só o verso da última folha sem marca. Nada é impresso se o
    /// pedido vier com uma delas.
    pub fn imprimir_duas_faces(&mut self, paginas: &[Pagina]) -> Result<(), ErroSimulacao> {
        self.exige(Estado::Ocioso)?;
        if paginas
            .iter()
            .any(|pagina| pagina.numero.is_none() || pagina.girar_180)
        {
            return Err(ErroSimulacao::DuasFacesRecusadas);
        }
        let marca = |pagina: Option<&Pagina>| Marca {
            pagina: pagina.and_then(|pagina| pagina.numero),
            girada: false,
        };
        for par in paginas.chunks(2) {
            self.pilha.push(Folha {
                face_a: Some(marca(par.first())),
                face_b: Some(marca(par.get(1))),
                face_a_para_cima: !self.mecanica.saida_face_impressa_para_cima,
                girada: false,
                sobreimpressao: None,
            });
        }
        if !self.mecanica.saida_no_topo {
            self.pilha.reverse();
        }
        self.estado = Estado::Concluido;
        Ok(())
    }

    pub fn recolocar(&mut self, gesto: Recolocacao) -> Result<(), ErroSimulacao> {
        self.exige(Estado::AguardandoRecolocacao)?;
        if gesto.retirar_folhas > self.pilha.len() {
            return Err(ErroSimulacao::QuantidadeFolhas {
                esperadas: gesto.retirar_folhas,
                disponiveis: self.pilha.len(),
            });
        }
        self.pilha.truncate(self.pilha.len() - gesto.retirar_folhas);
        if gesto.virar_maco {
            self.pilha.reverse();
            for folha in &mut self.pilha {
                folha.face_a_para_cima = !folha.face_a_para_cima;
            }
        }
        if gesto.inverter_ordem {
            self.pilha.reverse();
        }
        if gesto.girar_180 {
            for folha in &mut self.pilha {
                folha.girada = !folha.girada;
            }
        }
        self.estado = Estado::ProntoParaVerso;
        Ok(())
    }

    pub fn imprimir_verso(&mut self, paginas: &[Pagina]) -> Result<(), ErroSimulacao> {
        self.exige(Estado::ProntoParaVerso)?;
        if paginas.len() != self.pilha.len() {
            return Err(ErroSimulacao::QuantidadeFolhas {
                esperadas: paginas.len(),
                disponiveis: self.pilha.len(),
            });
        }
        let mut saida = Vec::with_capacity(self.pilha.len());
        if !self.mecanica.entrada_pelo_topo {
            self.pilha.reverse();
        }
        for pagina in paginas {
            // O tamanho foi conferido antes de iniciar a passagem.
            let Some(mut folha) = self.pilha.pop() else {
                return Err(ErroSimulacao::QuantidadeFolhas {
                    esperadas: paginas.len(),
                    disponiveis: 0,
                });
            };
            let marca = Marca {
                pagina: pagina.numero,
                girada: pagina.girar_180 ^ folha.girada ^ self.mecanica.giro_segunda_passada,
            };
            let imprime_a = folha.face_a_para_cima == self.mecanica.imprime_face_para_cima;
            let face = if imprime_a {
                &mut folha.face_a
            } else {
                &mut folha.face_b
            };
            if face.is_some() {
                folha.sobreimpressao = Some((imprime_a, marca));
            } else {
                *face = Some(marca);
            }
            folha.face_a_para_cima = if self.mecanica.saida_face_impressa_para_cima {
                imprime_a
            } else {
                !imprime_a
            };
            saida.push(folha);
        }
        if !self.mecanica.saida_no_topo {
            saida.reverse();
        }
        self.pilha = saida;
        self.estado = Estado::Concluido;
        Ok(())
    }
}

/// Motivo pelo qual uma folha final não está correta.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Defeito {
    /// As páginas das duas faces não são o par esperado para aquela folha.
    ParNaoEsperado,
    /// Uma face recebeu uma segunda impressão em vez da face vazia.
    Sobreposta,
    /// O verso tem orientação diferente da frente.
    VersoDeCabecaParaBaixo,
    /// As duas faces estão giradas: a folha inteira está de cabeça para baixo.
    FolhaDeCabecaParaBaixo,
}

impl Defeito {
    pub const fn descricao(self) -> &'static str {
        match self {
            Self::ParNaoEsperado => "as páginas não formam o par esperado",
            Self::Sobreposta => "uma face recebeu duas impressões",
            Self::VersoDeCabecaParaBaixo => "o verso está de cabeça para baixo",
            Self::FolhaDeCabecaParaBaixo => "a folha está de cabeça para baixo",
        }
    }
}

/// Resultado do julgamento de uma folha final.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum Veredito {
    Certa,
    Errada(Vec<Defeito>),
}

/// Ordena o par de páginas de uma folha; a página em branco fica por último.
fn ordenar_par(par: (Option<usize>, Option<usize>)) -> (Option<usize>, Option<usize>) {
    match par {
        (None, Some(_)) => (par.1, par.0),
        (Some(a), Some(b)) if a > b => (par.1, par.0),
        _ => par,
    }
}

/// Julga cada folha da pilha, na mesma ordem em que aparece em `pilha`.
///
/// `pares_esperados` lista o par de páginas de cada folha do lote, sem repetição.
/// Uma folha está certa quando as duas faces formam um par ainda não usado por
/// outra folha, nenhuma face recebeu segunda impressão, as duas faces têm a
/// mesma orientação e nenhuma está girada. Uma folha sem as duas faces impressas
/// não forma par.
pub fn conferir_folhas(
    pilha: &[Folha],
    pares_esperados: &[(Option<usize>, Option<usize>)],
) -> Vec<Veredito> {
    let mut restantes: Vec<_> = pares_esperados
        .iter()
        .map(|par| ordenar_par(*par))
        .collect();
    pilha
        .iter()
        .map(|folha| {
            let mut defeitos = Vec::new();
            let par = match (folha.face_a, folha.face_b) {
                (Some(a), Some(b)) => {
                    if a.girada && b.girada {
                        defeitos.push(Defeito::FolhaDeCabecaParaBaixo);
                    } else if a.girada != b.girada {
                        defeitos.push(Defeito::VersoDeCabecaParaBaixo);
                    }
                    Some(ordenar_par((a.pagina, b.pagina)))
                }
                _ => None,
            };
            if folha.sobreimpressao.is_some() {
                defeitos.push(Defeito::Sobreposta);
            }
            let casou = par
                .and_then(|p| restantes.iter().position(|r| *r == p))
                .map(|posicao| restantes.swap_remove(posicao))
                .is_some();
            if !casou {
                defeitos.insert(0, Defeito::ParNaoEsperado);
            }
            if defeitos.is_empty() {
                Veredito::Certa
            } else {
                Veredito::Errada(defeitos)
            }
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use plots_core::{
        ajustar_perfil_por_calibracao, plano_documento, OrdemPassadas, PerfilImpressora,
    };

    fn executar(
        total: usize,
        mecanismo: Mecanica,
        perfil: PerfilImpressora,
        gesto: Recolocacao,
    ) -> Simulador {
        let lote = plano_documento(total, 10, OrdemPassadas::default(), perfil)
            .unwrap()
            .remove(0);
        let mut sim = Simulador::novo(mecanismo);
        sim.imprimir_frente(&lote.passada1).unwrap();
        sim.recolocar(gesto).unwrap();
        sim.imprimir_verso(&lote.passada2).unwrap();
        sim
    }

    #[test]
    fn calibracao_corrige_pareamento_e_orientacao_nas_duas_mecanicas() {
        // Alimentação determinística e giro uniforme para todas as folhas;
        // sem sobreimpressão, perda ou mudança de gesto entre os dois testes.
        // Cada máquina usa o gesto que lhe cabe: a laser não vira o maço.
        for (mecanismo_base, gesto) in [
            (
                Mecanica::traseira(),
                Recolocacao {
                    virar_maco: true,
                    ..Default::default()
                },
            ),
            (Mecanica::gaveta(), Recolocacao::default()),
        ] {
            for giro_maquina in [false, true] {
                let mecanismo = Mecanica {
                    giro_segunda_passada: giro_maquina,
                    ..mecanismo_base
                };
                for inversa in [false, true] {
                    for giro in [false, true] {
                        let perfil = PerfilImpressora {
                            passada2_inversa: inversa,
                            verso_girar_180: giro,
                            ..Default::default()
                        };
                        let resultado = executar(4, mecanismo, perfil, gesto);
                        let leitura = resultado.folhas_para_leitura();
                        let folha_1 = leitura
                            .iter()
                            .find(|f| f.face_b.unwrap().pagina == Some(1))
                            .unwrap();
                        let folha_2 = leitura
                            .iter()
                            .find(|f| f.face_a.unwrap().pagina == Some(2))
                            .unwrap();
                        let par_correto = folha_1.face_a.unwrap().pagina == Some(2);
                        let invertida =
                            folha_1.face_b.unwrap().girada ^ folha_2.face_a.unwrap().girada;
                        let ajustado =
                            ajustar_perfil_por_calibracao(perfil, par_correto, invertida);
                        let finalizado = executar(4, mecanismo, ajustado, gesto);
                        for folha in &finalizado.folhas_para_leitura() {
                            let impar = folha.face_b.unwrap();
                            let par = folha.face_a.unwrap();
                            assert_eq!(par.pagina, impar.pagina.map(|n| n + 1));
                            assert_eq!(impar.girada, par.girada);
                        }
                    }
                }
            }
        }
    }

    #[test]
    fn duas_maquinas_com_perfis_coerentes_preservam_pares() {
        for total in [1, 3, 6] {
            for (mecanismo, perfil, gesto) in [
                (Mecanica::traseira(), perfil_jato(), virada_de_cima()),
                (Mecanica::gaveta(), perfil_laser(), Recolocacao::default()),
            ] {
                let sim = executar(total, mecanismo, perfil, gesto);
                let pares: Vec<_> = sim
                    .folhas_para_leitura()
                    .iter()
                    .rev()
                    .map(|f| (f.face_b.unwrap().pagina.unwrap(), f.face_a.unwrap().pagina))
                    .collect();
                let esperado: Vec<_> = (1..=total)
                    .step_by(2)
                    .map(|n| (n, (n < total).then_some(n + 1)))
                    .collect();
                assert_eq!(pares, esperado);
            }
        }
    }

    #[test]
    fn lotes_e_perfis_coerentes_preservam_folhas_e_orientacao_em_varias_mecanicas() {
        // Modelo de referência do teste: não calcula a ordem da pilha a partir
        // das flags do planejador. Em vez disso, observa as duas marcas de cada
        // folha depois do ciclo físico e exige o resultado legível pelo usuário.
        // Os quatro booleanos mecânicos independentes são enumerados; para cada
        // mecanismo, procura-se um perfil/gesto que passe um lote de calibração.
        let mut verificadas = [0usize; 8];
        let mut puladas = [0usize; 8];
        for mascara_perfil in 0u8..8 {
            let perfil = PerfilImpressora {
                passada1_inversa: mascara_perfil & 1 != 0,
                passada2_inversa: mascara_perfil & 2 != 0,
                verso_girar_180: mascara_perfil & 4 != 0,
                ..Default::default()
            };
            for mascara_mecanica in 0u8..32 {
                let mecanismo = Mecanica {
                    saida_face_impressa_para_cima: mascara_mecanica & 1 != 0,
                    saida_no_topo: mascara_mecanica & 2 != 0,
                    imprime_face_para_cima: mascara_mecanica & 4 != 0,
                    entrada_pelo_topo: mascara_mecanica & 8 != 0,
                    giro_segunda_passada: mascara_mecanica & 16 != 0,
                };

                let mut gesto_coerente = None;
                'gestos: for virar_maco in [false, true] {
                    for inverter_ordem in [false, true] {
                        for girar_180 in [false, true] {
                            let gesto = Recolocacao {
                                virar_maco,
                                inverter_ordem,
                                girar_180,
                                retirar_folhas: 0,
                            };
                            let piloto = executar(4, mecanismo, perfil, gesto);
                            if lote_correto(&piloto.folhas_para_leitura(), 1, 2, 4) {
                                gesto_coerente = Some(gesto);
                                break 'gestos;
                            }
                        }
                    }
                }
                let Some(gesto) = gesto_coerente else {
                    // Mecânica sem gesto que aprume o lote-piloto: fica contada
                    // como pulada e a contagem final afirma o número exato.
                    puladas[mascara_perfil as usize] += 1;
                    continue;
                };
                verificadas[mascara_perfil as usize] += 1;

                for total in 1..=200 {
                    for folhas_por_lote in 1..=50 {
                        let lotes = plano_documento(
                            total,
                            folhas_por_lote,
                            OrdemPassadas::default(),
                            perfil,
                        )
                        .unwrap();
                        let mut primeira_pagina = 1;
                        for lote in lotes {
                            let folhas_esperadas =
                                ((total - primeira_pagina + 2) / 2).min(folhas_por_lote);
                            let resultado =
                                executar_lote(&lote.passada1, &lote.passada2, mecanismo, gesto);
                            assert!(
                                lote_correto(
                                    &resultado.folhas_para_leitura(),
                                    primeira_pagina,
                                    folhas_esperadas,
                                    total,
                                ),
                                "perfil {mascara_perfil}, mecânica {mascara_mecanica}, total {total}, lote {folhas_por_lote}"
                            );
                            primeira_pagina += folhas_esperadas * 2;
                        }
                    }
                }
            }
            let (v, p) = (
                verificadas[mascara_perfil as usize],
                puladas[mascara_perfil as usize],
            );
            assert_eq!(
                v + p,
                32,
                "perfil {mascara_perfil}: toda mecânica deve ser verificada ou pulada"
            );
            // A conferência compara as faces entre si: a rotação do gesto passa a ser um
            // ajuste que compensa o verso, e metade das mecânicas ganha um gesto que aprume o lote.
            assert_eq!(
                (v, p),
                (16, 16),
                "perfil {mascara_perfil}: cobertura mudou (verificadas, puladas)"
            );
        }
    }

    fn executar_lote(
        frente: &[Pagina],
        verso: &[Pagina],
        mecanismo: Mecanica,
        gesto: Recolocacao,
    ) -> Simulador {
        let mut sim = Simulador::novo(mecanismo);
        sim.imprimir_frente(frente).unwrap();
        sim.recolocar(gesto).unwrap();
        sim.imprimir_verso(verso).unwrap();
        sim
    }

    fn lote_correto(
        pilha: &[Folha],
        primeira_pagina: usize,
        quantidade: usize,
        total: usize,
    ) -> bool {
        // A pilha interna vai de baixo para cima; vista pela ordem em que as
        // folhas seriam retiradas, cada número ímpar tem o par imediatamente
        // seguinte no verso (ou branco final) e as duas artes ficam aprumadas.
        let folhas: Vec<_> = pilha.iter().rev().collect();
        if folhas.len() != quantidade {
            return false;
        }
        folhas.iter().enumerate().all(|(indice, folha)| {
            let frente = folha.face_a;
            let verso = folha.face_b;
            let esperado_verso = Some(primeira_pagina + indice * 2);
            let pagina_frente = primeira_pagina + indice * 2 + 1;
            let esperado_frente = (pagina_frente <= total).then_some(pagina_frente);
            let marcas = [frente, verso];
            let marca_frente = marcas
                .iter()
                .flatten()
                .find(|m| m.pagina == esperado_frente);
            let marca_verso = marcas.iter().flatten().find(|m| m.pagina == esperado_verso);
            let frente_em_branco_valida = esperado_frente.is_none()
                && indice + 1 == folhas.len()
                && marcas.iter().flatten().any(|m| m.pagina.is_none());
            let faces_corretas =
                (marca_frente.is_some() || frente_em_branco_valida) && marca_verso.is_some();
            let marca_frente_final = marca_frente.or_else(|| {
                if frente_em_branco_valida {
                    marcas.iter().flatten().find(|m| m.pagina.is_none())
                } else {
                    None
                }
            });
            // As duas faces precisam estar sem giro. Uma virada pela borda de cima deixa a
            // folha girada em relação ao maço; o verso girado compensa, e a conferência
            // vale para a relação entre as faces, não para a posição do maço.
            let orientacao_correta = marca_frente_final
                .zip(marca_verso)
                .is_some_and(|(a, b)| !a.girada && !b.girada);
            faces_corretas && orientacao_correta && folha.sobreimpressao.is_none()
        })
    }

    fn paginas_de(numeros: &[usize]) -> Vec<Pagina> {
        numeros
            .iter()
            .map(|n| Pagina {
                numero: Some(*n),
                girar_180: false,
            })
            .collect()
    }

    #[test]
    fn gaveta_empilha_cada_folha_em_cima_da_anterior_com_a_face_impressa_para_baixo() {
        let mut sim = Simulador::novo(Mecanica::gaveta());
        sim.imprimir_frente(&paginas_de(&[2, 4, 6])).unwrap();
        let paginas: Vec<_> = sim
            .pilha()
            .iter()
            .map(|f| f.face_a.and_then(|m| m.pagina))
            .collect();
        // De baixo para cima: a primeira folha impressa fica embaixo.
        assert_eq!(paginas, vec![Some(2), Some(4), Some(6)]);
        assert!(sim.pilha().iter().all(|f| !f.face_a_para_cima));
    }

    #[test]
    fn gaveta_alimenta_a_folha_de_cima_da_pilha() {
        let folha = |pagina: usize| Folha {
            face_a: Some(Marca {
                pagina: Some(pagina),
                girada: false,
            }),
            face_b: None,
            face_a_para_cima: false,
            girada: false,
            sobreimpressao: None,
        };
        let mut sim = Simulador::novo(Mecanica::gaveta());
        // Pilha montada à mão, de baixo para cima: a folha 2 e depois a folha 4.
        sim.pilha = vec![folha(2), folha(4)];
        sim.estado = Estado::AguardandoRecolocacao;
        // Sem virar o maço, a face impressa continua para baixo; a gaveta imprime
        // a face de cima, que está vazia, e nenhuma folha recebe sobreimpressão.
        sim.recolocar(Recolocacao::default()).unwrap();
        sim.imprimir_verso(&paginas_de(&[1, 3])).unwrap();
        let por_folha: Vec<_> = sim
            .pilha()
            .iter()
            .map(|f| {
                (
                    f.face_a.and_then(|m| m.pagina),
                    f.face_b.and_then(|m| m.pagina),
                )
            })
            .collect();
        // A folha de cima (4) é puxada primeiro e recebe a página 1.
        assert!(por_folha.contains(&(Some(4), Some(1))), "{por_folha:?}");
        assert!(por_folha.contains(&(Some(2), Some(3))), "{por_folha:?}");
        assert!(sim.pilha().iter().all(|f| f.sobreimpressao.is_none()));
    }

    #[test]
    fn gaveta_ciclo_completo_sai_com_a_ultima_folha_em_cima_e_le_se_virado() {
        for total in [4usize, 5] {
            for ordem in [OrdemPassadas::ParesPrimeiro, OrdemPassadas::ImparesPrimeiro] {
                let lote = plano_documento(total, 10, ordem, perfil_laser())
                    .unwrap()
                    .remove(0);
                let sim = executar_lote(
                    &lote.passada1,
                    &lote.passada2,
                    Mecanica::gaveta(),
                    Recolocacao::default(),
                );
                // Na saída, a primeira folha impressa está embaixo.
                let saida = sim.pilha();
                assert!(saida.first().is_some_and(|f| lote_correto(
                    &[f.clone()],
                    1,
                    1,
                    total.min(2)
                )));
                // Virado o maço, a folha 1|2 fica por cima e nenhuma está invertida.
                assert!(lote_correto(
                    &sim.folhas_para_leitura(),
                    1,
                    total.div_ceil(2),
                    total
                ));
            }
        }
    }

    #[test]
    fn nao_virar_maco_registra_sobreimpressao() {
        let sim = executar(
            2,
            Mecanica::traseira(),
            PerfilImpressora::default(),
            Recolocacao::default(),
        );
        assert_eq!(sim.pilha()[0].face_a.unwrap().pagina, Some(2));
        assert_eq!(sim.pilha()[0].sobreimpressao.unwrap().1.pagina, Some(1));
        assert_eq!(sim.pilha()[0].face_b, None);
    }

    #[test]
    fn giro_da_recolocacao_aparece_no_verso() {
        let sim = executar(
            2,
            Mecanica::traseira(),
            PerfilImpressora::default(),
            Recolocacao {
                virar_maco: true,
                girar_180: true,
                ..Default::default()
            },
        );
        assert!(sim.pilha()[0].face_b.unwrap().girada);
    }

    #[test]
    fn folha_ausente_e_ordem_de_operacoes_invalidas_sao_rejeitadas() {
        let lote = plano_documento(4, 2, OrdemPassadas::default(), PerfilImpressora::default())
            .unwrap()
            .remove(0);
        let mut sim = Simulador::novo(Mecanica::traseira());
        assert!(matches!(
            sim.imprimir_verso(&lote.passada2),
            Err(ErroSimulacao::EstadoInvalido { .. })
        ));
        sim.imprimir_frente(&lote.passada1).unwrap();
        sim.recolocar(Recolocacao {
            virar_maco: true,
            retirar_folhas: 1,
            ..Default::default()
        })
        .unwrap();
        assert_eq!(
            sim.imprimir_verso(&lote.passada2),
            Err(ErroSimulacao::QuantidadeFolhas {
                esperadas: 2,
                disponiveis: 1
            })
        );
        assert_eq!(sim.estado(), Estado::ProntoParaVerso);
    }

    #[test]
    fn permite_novo_lote_apos_retirar_o_anterior() {
        let lote = plano_documento(2, 1, OrdemPassadas::default(), PerfilImpressora::default())
            .unwrap()
            .remove(0);
        let mut sim = Simulador::novo(Mecanica::traseira());
        assert!(sim.retirar_lote().is_err());
        for _ in 0..2 {
            sim.imprimir_frente(&lote.passada1).unwrap();
            sim.recolocar(Recolocacao {
                virar_maco: true,
                ..Default::default()
            })
            .unwrap();
            sim.imprimir_verso(&lote.passada2).unwrap();
            assert_eq!(sim.retirar_lote().unwrap().len(), 1);
            assert_eq!(sim.estado(), Estado::Ocioso);
        }
    }

    #[test]
    fn falta_de_folha_permite_cancelar_e_reimprimir_lote() {
        let perfil = PerfilImpressora {
            passada1_inversa: true,
            passada2_inversa: true,
            ..Default::default()
        };
        let lote = plano_documento(4, 2, OrdemPassadas::default(), perfil)
            .unwrap()
            .remove(0);
        let mut sim = Simulador::novo(Mecanica::traseira());
        sim.imprimir_frente(&lote.passada1).unwrap();
        sim.recolocar(Recolocacao {
            virar_maco: true,
            retirar_folhas: 1,
            ..Default::default()
        })
        .unwrap();
        assert!(matches!(
            sim.imprimir_verso(&lote.passada2),
            Err(ErroSimulacao::QuantidadeFolhas { .. })
        ));
        assert_eq!(sim.cancelar_lote().unwrap().len(), 1);
        sim.imprimir_frente(&lote.passada1).unwrap();
        sim.recolocar(Recolocacao {
            virar_maco: true,
            ..Default::default()
        })
        .unwrap();
        sim.imprimir_verso(&lote.passada2).unwrap();
        let pares: Vec<_> = sim
            .pilha()
            .iter()
            .rev()
            .map(|f| (f.face_b.unwrap().pagina, f.face_a.unwrap().pagina))
            .collect();
        assert_eq!(pares, vec![(Some(1), Some(2)), (Some(3), Some(4))]);
    }

    fn marca_de_teste(pagina: Option<usize>, girada: bool) -> Option<Marca> {
        Some(Marca { pagina, girada })
    }

    fn folha_de_teste(face_a: Option<Marca>, face_b: Option<Marca>, sobreposta: bool) -> Folha {
        Folha {
            face_a,
            face_b,
            face_a_para_cima: true,
            girada: false,
            sobreimpressao: sobreposta.then_some((
                true,
                Marca {
                    pagina: Some(99),
                    girada: false,
                },
            )),
        }
    }

    #[test]
    fn conferir_folhas_separa_par_orientacao_e_sobreposicao() {
        let pilha = [
            folha_de_teste(
                marca_de_teste(Some(2), false),
                marca_de_teste(Some(1), false),
                false,
            ),
            folha_de_teste(
                marca_de_teste(Some(3), false),
                marca_de_teste(Some(4), true),
                false,
            ),
            folha_de_teste(
                marca_de_teste(Some(5), false),
                marca_de_teste(None, false),
                false,
            ),
            folha_de_teste(
                marca_de_teste(Some(6), false),
                marca_de_teste(Some(7), false),
                true,
            ),
            folha_de_teste(
                marca_de_teste(Some(2), false),
                marca_de_teste(Some(1), false),
                false,
            ),
            folha_de_teste(marca_de_teste(Some(9), false), None, true),
        ];
        let esperados = [
            (Some(1), Some(2)),
            (Some(3), Some(4)),
            (Some(5), None),
            (Some(6), Some(7)),
        ];
        assert_eq!(
            conferir_folhas(&pilha, &esperados),
            vec![
                Veredito::Certa,
                Veredito::Errada(vec![Defeito::VersoDeCabecaParaBaixo]),
                Veredito::Certa,
                Veredito::Errada(vec![Defeito::Sobreposta]),
                // Mesmo par já usado pela primeira folha: não conta de novo.
                Veredito::Errada(vec![Defeito::ParNaoEsperado]),
                Veredito::Errada(vec![Defeito::ParNaoEsperado, Defeito::Sobreposta]),
            ]
        );
    }

    #[test]
    fn conferir_folhas_aceita_par_em_qualquer_ordem_e_branco_final() {
        let pilha = [folha_de_teste(
            marca_de_teste(None, false),
            marca_de_teste(Some(5), false),
            false,
        )];
        assert_eq!(
            conferir_folhas(&pilha, &[(Some(5), None)]),
            vec![Veredito::Certa]
        );
        assert_eq!(
            conferir_folhas(&pilha, &[(Some(4), None)]),
            vec![Veredito::Errada(vec![Defeito::ParNaoEsperado])]
        );
    }

    /// Gesto da virada pela lateral: vira o maço e nada mais.
    const GESTO_LATERAL: Recolocacao = Recolocacao {
        virar_maco: true,
        inverter_ordem: false,
        girar_180: false,
        retirar_folhas: 0,
    };

    /// Gesto sem nenhuma operação: o maço não é tocado na recolocação.
    const GESTO_NENHUM: Recolocacao = Recolocacao {
        virar_maco: false,
        inverter_ordem: false,
        girar_180: false,
        retirar_folhas: 0,
    };

    /// Máquina hipotética que imprime a face de baixo e sai pelo topo, com a face
    /// impressa para baixo. Mantém os valores das linhas hipotéticas da tabela.
    const fn maquina_que_imprime_a_face_de_baixo() -> Mecanica {
        Mecanica {
            imprime_face_para_cima: false,
            ..Mecanica::gaveta()
        }
    }

    /// Perfis de (passada1_inversa, passada2_inversa, verso_girar_180), cada um com a máquina
    /// e o gesto de recolocação que o produzem, lendo o maço final como em
    /// `folhas_para_leitura`. As linhas hipotéticas, que não foram medidas, seguem a regra
    /// física das passadas: a primeira inverte quando a entrada pelo topo e a face impressa
    /// para cima coincidem; a segunda inverte quando a saída no topo e a face impressa para
    /// cima coincidem; e o verso só gira quando a máquina gira a segunda passada sozinha. Todas
    /// têm caminho reto e viram como livro. As duas últimas linhas são o jato de tinta e a laser,
    /// com os gestos dos modelos: a virada de bloco de notas e a curva em U pedem o verso girado.
    const TABELA_DE_MECANICAS: [((bool, bool, bool), Mecanica, Recolocacao); 10] = [
        // Imprime a face de baixo e sai no topo, face para baixo.
        (
            (false, false, false),
            maquina_que_imprime_a_face_de_baixo(),
            GESTO_LATERAL,
        ),
        // Igual, mas devolve cada folha por baixo do maço: a segunda passada inverte.
        (
            (false, true, false),
            Mecanica {
                saida_no_topo: false,
                ..maquina_que_imprime_a_face_de_baixo()
            },
            GESTO_LATERAL,
        ),
        // Traseira que devolve cada folha por baixo do maço: a segunda passada não inverte.
        (
            (true, false, false),
            Mecanica {
                saida_no_topo: false,
                ..Mecanica::traseira()
            },
            GESTO_LATERAL,
        ),
        // Traseira: entra pelo topo, imprime com a face para cima e sai por cima do maço.
        ((true, true, false), Mecanica::traseira(), GESTO_LATERAL),
        // Gira a segunda passada: o verso é girado 180° para compensar.
        (
            (false, false, true),
            Mecanica {
                giro_segunda_passada: true,
                ..maquina_que_imprime_a_face_de_baixo()
            },
            GESTO_LATERAL,
        ),
        // Traseira que devolve por baixo e gira a segunda passada: o verso é compensado.
        (
            (true, false, true),
            Mecanica {
                saida_no_topo: false,
                giro_segunda_passada: true,
                ..Mecanica::traseira()
            },
            GESTO_LATERAL,
        ),
        // Devolve por baixo e gira a segunda passada: o verso é compensado.
        (
            (false, true, true),
            Mecanica {
                saida_no_topo: false,
                giro_segunda_passada: true,
                ..maquina_que_imprime_a_face_de_baixo()
            },
            GESTO_LATERAL,
        ),
        // Traseira que gira a segunda passada: o verso é compensado.
        (
            (true, true, true),
            Mecanica {
                giro_segunda_passada: true,
                ..Mecanica::traseira()
            },
            GESTO_LATERAL,
        ),
        // Jato de tinta: perfil e gesto do modelo.
        (
            (true, true, true),
            Mecanica::traseira(),
            Recolocacao::virada_pela_borda_de_cima(),
        ),
        // Laser: perfil e gesto do modelo.
        ((true, false, true), Mecanica::gaveta(), GESTO_NENHUM),
    ];

    fn perfil_de(
        passada1_inversa: bool,
        passada2_inversa: bool,
        verso_girar_180: bool,
    ) -> PerfilImpressora {
        PerfilImpressora {
            passada1_inversa,
            passada2_inversa,
            verso_girar_180,
            ..Default::default()
        }
    }

    /// Confere o maço inteiro de um documento: cada lote deve sair com as
    /// folhas certas, na ordem 1|2, 3|4…, e sem nenhuma folha de cabeça para baixo.
    fn maco_confere(
        total: usize,
        folhas_por_lote: usize,
        ordem: OrdemPassadas,
        perfil: PerfilImpressora,
        mecanismo: Mecanica,
        gesto: Recolocacao,
    ) -> Result<(), String> {
        let lotes =
            plano_documento(total, folhas_por_lote, ordem, perfil).map_err(|e| e.to_string())?;
        let mut primeira_pagina = 1;
        for lote in lotes {
            let folhas_esperadas =
                ((total.saturating_sub(primeira_pagina) + 2) / 2).min(folhas_por_lote);
            let resultado = executar_lote(&lote.passada1, &lote.passada2, mecanismo, gesto);
            if !lote_correto(
                &resultado.folhas_para_leitura(),
                primeira_pagina,
                folhas_esperadas,
                total,
            ) {
                return Err(format!(
                    "lote a partir da página {primeira_pagina} com {folhas_esperadas} folha(s) saiu errado"
                ));
            }
            primeira_pagina += folhas_esperadas * 2;
        }
        Ok(())
    }

    proptest::proptest! {
        #![proptest_config(proptest::test_runner::Config {
            cases: 4000,
            failure_persistence: None,
            ..proptest::test_runner::Config::default()
        })]

        #[test]
        fn maco_final_fica_em_ordem_para_cada_perfil_com_mecanica_coerente(
            indice_perfil in 0usize..TABELA_DE_MECANICAS.len(),
            total in 1usize..=200,
            folhas_por_lote in 1usize..=50,
        ) {
            let ((p1, p2, girar), mecanismo, gesto) = TABELA_DE_MECANICAS[indice_perfil];
            let perfil = perfil_de(p1, p2, girar);
            let resultado = maco_confere(
                total,
                folhas_por_lote,
                OrdemPassadas::default(),
                perfil,
                mecanismo,
                gesto,
            );
            let motivo = resultado.err().unwrap_or_default();
            proptest::prop_assert!(
                motivo.is_empty(),
                "perfil {indice_perfil}, mecânica {mecanismo:?}, gesto {gesto:?}: {motivo}"
            );
        }
    }

    #[test]
    fn tabela_cobre_os_oito_perfis_e_inclui_os_dois_modelos() {
        for mascara in 0u8..8 {
            let perfil = (mascara & 1 != 0, mascara & 2 != 0, mascara & 4 != 0);
            let linhas = TABELA_DE_MECANICAS
                .iter()
                .filter(|(p, _, _)| *p == perfil)
                .count();
            assert!(linhas >= 1, "perfil {mascara}");
        }
        assert!(TABELA_DE_MECANICAS.contains(&(
            (true, true, true),
            Mecanica::traseira(),
            Recolocacao::virada_pela_borda_de_cima()
        )));
        assert!(TABELA_DE_MECANICAS.contains(&(
            (true, false, true),
            Mecanica::gaveta(),
            GESTO_NENHUM
        )));
    }

    #[test]
    fn lote_piloto_de_cada_linha_da_tabela_aprumado_com_o_gesto_da_linha() {
        for ((p1, p2, girar), mecanismo, gesto) in TABELA_DE_MECANICAS {
            let sim = executar(4, mecanismo, perfil_de(p1, p2, girar), gesto);
            assert!(
                lote_correto(&sim.folhas_para_leitura(), 1, 2, 4),
                "perfil {:?}, mecânica {mecanismo:?}, gesto {gesto:?}",
                (p1, p2, girar)
            );
        }
    }

    #[test]
    fn ordem_impares_com_os_dois_modelos_mantem_folhas_certas() {
        for (mecanismo, perfil, gesto) in [
            (Mecanica::traseira(), perfil_jato(), virada_de_cima()),
            (Mecanica::gaveta(), perfil_laser(), GESTO_NENHUM),
        ] {
            for total in 1..=40 {
                for folhas in 1..=6 {
                    let resultado = maco_confere(
                        total,
                        folhas,
                        OrdemPassadas::ImparesPrimeiro,
                        perfil,
                        mecanismo,
                        gesto,
                    );
                    assert!(
                        resultado.is_ok(),
                        "total {total}, lote {folhas}: {resultado:?}"
                    );
                }
            }
        }
    }

    /// Perfil da laser: frente inversa, verso direto e girado.
    fn perfil_laser() -> PerfilImpressora {
        ModeloImpressora::Laser.perfil()
    }

    /// Perfil do jato de tinta: frente e verso inversos, com o verso girado.
    fn perfil_jato() -> PerfilImpressora {
        ModeloImpressora::JatoDeTinta.perfil()
    }

    /// Virada pela borda de cima: virar o maço e girar cada folha em 180°.
    fn virada_de_cima() -> Recolocacao {
        Recolocacao {
            virar_maco: true,
            girar_180: true,
            ..Default::default()
        }
    }

    fn numeros(paginas: &[Pagina]) -> Vec<Option<usize>> {
        paginas.iter().map(|p| p.numero).collect()
    }

    #[test]
    fn laser_imprime_a_face_que_fica_para_cima() {
        let mut sim = Simulador::novo(Mecanica::gaveta());
        sim.imprimir_frente(&paginas_de(&[2])).unwrap();
        sim.recolocar(Recolocacao::default()).unwrap();
        sim.imprimir_verso(&paginas_de(&[1])).unwrap();
        let folha = sim.pilha()[0].clone();
        assert_eq!(folha.face_a.and_then(|m| m.pagina), Some(2));
        assert_eq!(folha.face_b.and_then(|m| m.pagina), Some(1));
        assert_eq!(folha.sobreimpressao, None);
    }

    #[test]
    fn laser_seis_paginas_frente_6_4_2_verso_1_3_5_girado_sem_virar() {
        let lote = plano_documento(6, 3, OrdemPassadas::ParesPrimeiro, perfil_laser())
            .unwrap()
            .remove(0);
        assert_eq!(numeros(&lote.passada1), vec![Some(6), Some(4), Some(2)]);
        assert_eq!(numeros(&lote.passada2), vec![Some(1), Some(3), Some(5)]);
        assert!(lote.passada2.iter().all(|p| p.girar_180));
        let sim = executar(
            6,
            Mecanica::gaveta(),
            perfil_laser(),
            Recolocacao::default(),
        );
        assert!(sim.pilha().iter().all(|f| f.sobreimpressao.is_none()));
        assert!(lote_correto(&sim.folhas_para_leitura(), 1, 3, 6));
    }

    #[test]
    fn jato_seis_paginas_frente_6_4_2_verso_5_3_1_girado_com_virada_de_cima() {
        let lote = plano_documento(6, 3, OrdemPassadas::ParesPrimeiro, perfil_jato())
            .unwrap()
            .remove(0);
        assert_eq!(numeros(&lote.passada1), vec![Some(6), Some(4), Some(2)]);
        assert_eq!(numeros(&lote.passada2), vec![Some(5), Some(3), Some(1)]);
        assert!(lote.passada2.iter().all(|p| p.girar_180));
        let sim = executar(6, Mecanica::traseira(), perfil_jato(), virada_de_cima());
        let pilha = sim.folhas_para_leitura();
        let pares = [(Some(1), Some(2)), (Some(3), Some(4)), (Some(5), Some(6))];
        let veredictos = conferir_folhas(&pilha, &pares);
        assert!(
            veredictos.iter().all(|v| *v == Veredito::Certa),
            "{veredictos:?}"
        );
        assert!(lote_correto(&pilha, 1, 3, 6));
    }

    #[test]
    fn conferir_folhas_marca_folha_com_as_duas_faces_giradas() {
        let marca = |pagina| {
            Some(Marca {
                pagina: Some(pagina),
                girada: true,
            })
        };
        let pilha = [Folha {
            face_a: marca(2),
            face_b: marca(1),
            face_a_para_cima: true,
            girada: false,
            sobreimpressao: None,
        }];
        assert_eq!(
            conferir_folhas(&pilha, &[(Some(1), Some(2))]),
            vec![Veredito::Errada(vec![Defeito::FolhaDeCabecaParaBaixo])]
        );
    }

    #[test]
    fn folha_com_as_duas_faces_giradas_nao_confere_no_oraculo() {
        let marca = |pagina| {
            Some(Marca {
                pagina: Some(pagina),
                girada: true,
            })
        };
        let pilha = [Folha {
            face_a: marca(2),
            face_b: marca(1),
            face_a_para_cima: true,
            girada: true,
            sobreimpressao: None,
        }];
        assert!(!lote_correto(&pilha, 1, 1, 2));
    }

    /// Passa as 6 páginas pelo simulador do modelo, na sequência que o modelo manda enviar.
    fn seis_paginas_do_modelo(
        modelo: ModeloImpressora,
        frente: [usize; 3],
        verso: [usize; 3],
        verso_girado: bool,
    ) {
        let lotes = plots_core::plano_documento(
            6,
            3,
            plots_core::OrdemPassadas::ParesPrimeiro,
            modelo.perfil(),
        )
        .unwrap();
        let [lote] = lotes.as_slice() else {
            panic!("deveria sair um lote só");
        };
        let enviadas = |paginas: &[Pagina]| -> Vec<(usize, bool)> {
            paginas
                .iter()
                .map(|pagina| (pagina.numero.unwrap(), pagina.girar_180))
                .collect()
        };
        assert_eq!(enviadas(&lote.passada1), frente.map(|n| (n, false)));
        assert_eq!(enviadas(&lote.passada2), verso.map(|n| (n, verso_girado)));

        let mut sim = Simulador::novo(Mecanica::do_modelo(modelo));
        sim.imprimir_frente(&lote.passada1).unwrap();
        sim.recolocar(modelo.gesto().into()).unwrap();
        sim.imprimir_verso(&lote.passada2).unwrap();
        let pares = [(Some(1), Some(2)), (Some(3), Some(4)), (Some(5), Some(6))];
        let veredictos = conferir_folhas(&sim.folhas_para_leitura(), &pares);
        assert!(
            veredictos.iter().all(|v| matches!(v, Veredito::Certa)),
            "{modelo:?}: {veredictos:?}"
        );
    }

    #[test]
    fn jato_de_tinta_entrega_seis_paginas_certas_no_simulador() {
        seis_paginas_do_modelo(ModeloImpressora::JatoDeTinta, [6, 4, 2], [5, 3, 1], true);
    }

    #[test]
    fn laser_entrega_seis_paginas_certas_no_simulador() {
        seis_paginas_do_modelo(ModeloImpressora::Laser, [6, 4, 2], [1, 3, 5], true);
    }

    #[test]
    fn na_curva_em_u_o_verso_sem_giro_sai_de_cabeca_para_baixo() {
        let sem_giro = PerfilImpressora {
            verso_girar_180: false,
            ..perfil_laser()
        };
        let sim = executar(6, Mecanica::gaveta(), sem_giro, Recolocacao::default());
        let pares = [(Some(1), Some(2)), (Some(3), Some(4)), (Some(5), Some(6))];
        let veredictos = conferir_folhas(&sim.folhas_para_leitura(), &pares);
        assert_eq!(
            veredictos,
            vec![Veredito::Errada(vec![Defeito::VersoDeCabecaParaBaixo]); 3]
        );
    }

    #[test]
    fn dezesseis_mecanicas_e_vinte_e_quatro_perfis_seguem_a_regra_do_caminho_do_papel() {
        // Caminho reto: a folha só anda, então é preciso virar para achar a face em branco. Virar
        // como livro mantém a aresta que lidera; como bloco de notas, troca. Curva em U: a face em
        // branco já volta para o lado que recebe tinta, sem virar, e a aresta que lidera já trocou.
        for mascara in 0u8..16 {
            let mecanica = Mecanica {
                saida_face_impressa_para_cima: mascara & 1 != 0,
                saida_no_topo: mascara & 2 != 0,
                imprime_face_para_cima: mascara & 4 != 0,
                entrada_pelo_topo: mascara & 8 != 0,
                giro_segunda_passada: false,
            };
            let curva_em_u =
                mecanica.imprime_face_para_cima != mecanica.saida_face_impressa_para_cima;
            for virada in [
                ViradaMaco::NaoVirar,
                ViradaMaco::PelaLateral,
                ViradaMaco::PelaBordaDeCima,
            ] {
                let vira = virada != ViradaMaco::NaoVirar;
                for bits in 0u8..8 {
                    let perfil = PerfilImpressora {
                        passada1_inversa: bits & 1 != 0,
                        passada2_inversa: bits & 2 != 0,
                        verso_girar_180: bits & 4 != 0,
                        gesto_recolocar: GestoRecolocar {
                            virada,
                            ..GestoRecolocar::default()
                        },
                    };
                    let sim = executar(6, mecanica, perfil, perfil.gesto_recolocar.into());
                    let contexto = format!("{mecanica:?}, {perfil:?}");
                    let pilha = sim.folhas_para_leitura();
                    let acha_a_face_em_branco = vira != curva_em_u;
                    assert_eq!(
                        pilha.iter().all(|f| f.sobreimpressao.is_none()),
                        acha_a_face_em_branco,
                        "{contexto}"
                    );
                    if acha_a_face_em_branco {
                        let em_pe = pilha
                            .iter()
                            .all(|f| f.face_a.map(|m| m.girada) == f.face_b.map(|m| m.girada));
                        // Só a virada de livro, no caminho reto, mantém a aresta que lidera.
                        let pede_giro = virada != ViradaMaco::PelaLateral;
                        assert_eq!(em_pe, perfil.verso_girar_180 == pede_giro, "{contexto}");
                    }
                }
            }
        }
    }

    #[test]
    fn cada_virada_do_gesto_vira_a_recolocacao_do_simulador() {
        let gesto = |virada| GestoRecolocar {
            virada,
            ..GestoRecolocar::default()
        };
        assert_eq!(
            Recolocacao::from(gesto(ViradaMaco::NaoVirar)),
            Recolocacao::default()
        );
        assert_eq!(
            Recolocacao::from(gesto(ViradaMaco::PelaLateral)),
            Recolocacao {
                virar_maco: true,
                ..Recolocacao::default()
            }
        );
        assert_eq!(
            Recolocacao::from(gesto(ViradaMaco::PelaBordaDeCima)),
            Recolocacao::virada_pela_borda_de_cima()
        );
    }

    #[test]
    fn virada_de_cima_sem_girar_o_verso_deixa_a_folha_de_cabeca_para_baixo() {
        let sim = executar(
            6,
            Mecanica::traseira(),
            perfil_de(true, true, false),
            virada_de_cima(),
        );
        let pares = [(Some(1), Some(2)), (Some(3), Some(4)), (Some(5), Some(6))];
        let veredictos = conferir_folhas(&sim.folhas_para_leitura(), &pares);
        assert!(
            veredictos.iter().all(|v| matches!(
                v,
                Veredito::Errada(defeitos) if defeitos.contains(&Defeito::VersoDeCabecaParaBaixo)
            )),
            "{veredictos:?}"
        );
    }
}
pub mod fluxo;
pub mod job;
pub mod opcoes;
