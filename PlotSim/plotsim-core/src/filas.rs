//! Criar, listar e remover filas virtuais. Toda operação pode ser repetida sem efeito novo, e nada
//! fora do que o simulador criou é tocado.

use crate::modo::ModoFisico;
use crate::nome::{ErroNome, NomeVirtual};
use crate::pastas::{ErroPasta, Pastas, ARQUIVO_DA_PORTA};
use crate::registro::{ErroRegistro, Registro};
use crate::sistema::{ErroSistema, FilaDoSistema, SistemaDeFilas};
use std::fmt;
use std::fs;
use std::io;
use std::path::PathBuf;

/// Driver genérico que acompanha o Windows.
pub const DRIVER_DAS_FILAS: &str = "Microsoft Print To PDF";

/// Falhas do gerenciador.
#[derive(Debug)]
pub enum ErroFilas {
    Nome(ErroNome),
    Pasta(ErroPasta),
    Registro(ErroRegistro),
    Sistema(ErroSistema),
    /// Já existe uma fila com o nome, mas com outro driver ou outra porta: ela não é do simulador.
    FilaAlheia {
        fila: String,
        driver: String,
        porta: String,
    },
    /// Não deu para criar a pasta da porta.
    PastaDaPorta(PathBuf, io::Error),
    /// O registro está cheio. Nada foi criado.
    SemEspaco,
}

impl fmt::Display for ErroFilas {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Nome(erro) => erro.fmt(f),
            Self::Pasta(erro) => erro.fmt(f),
            Self::Registro(erro) => erro.fmt(f),
            Self::Sistema(erro) => erro.fmt(f),
            Self::FilaAlheia {
                fila,
                driver,
                porta,
            } => write!(
                f,
                "a fila \"{fila}\" já existe com o driver \"{driver}\" e a porta \"{porta}\"; ela não foi criada pelo PlotSim e não será alterada"
            ),
            Self::PastaDaPorta(pasta, erro) => {
                write!(f, "não deu para criar {}: {erro}", pasta.display())
            }
            Self::SemEspaco => f.write_str("o limite de impressoras virtuais foi atingido"),
        }
    }
}

impl std::error::Error for ErroFilas {}

impl From<ErroNome> for ErroFilas {
    fn from(erro: ErroNome) -> Self {
        Self::Nome(erro)
    }
}
impl From<ErroPasta> for ErroFilas {
    fn from(erro: ErroPasta) -> Self {
        Self::Pasta(erro)
    }
}
impl From<ErroRegistro> for ErroFilas {
    fn from(erro: ErroRegistro) -> Self {
        Self::Registro(erro)
    }
}
impl From<ErroSistema> for ErroFilas {
    fn from(erro: ErroSistema) -> Self {
        Self::Sistema(erro)
    }
}

/// Como uma fila conhecida está em relação ao sistema.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Situacao {
    /// Existe e está como o simulador a criou.
    Pronta,
    /// Está no registro, mas não no sistema.
    AusenteDoSistema,
    /// Tem o prefixo e está como o simulador cria, mas não está no registro: sem modo físico.
    SemRegistro,
    /// Tem o nome, mas outro driver ou outra porta. O simulador não mexe nela.
    Diferente,
}

/// Uma fila virtual na listagem.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct FilaVirtual {
    pub nome: NomeVirtual,
    pub porta: String,
    pub modo: Option<ModoFisico>,
    pub situacao: Situacao,
}

/// O que sobrou depois de uma remoção.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct Remocao {
    /// A fila existia e foi removida agora.
    pub fila_removida: bool,
    /// A porta existia e foi removida agora.
    pub porta_removida: bool,
}

/// Gerencia as filas virtuais em um sistema.
pub struct Filas<S> {
    sistema: S,
    pastas: Pastas,
}

fn mesmo_texto(a: &str, b: &str) -> bool {
    a.to_lowercase() == b.to_lowercase()
}

fn e_do_simulador(fila: &FilaDoSistema, porta: &str) -> bool {
    mesmo_texto(&fila.driver, DRIVER_DAS_FILAS) && mesmo_texto(&fila.porta, porta)
}

fn procurar<'a>(filas: &'a [FilaDoSistema], nome: &NomeVirtual) -> Option<&'a FilaDoSistema> {
    filas
        .iter()
        .find(|fila| NomeVirtual::da_fila(&fila.nome).is_ok_and(|outro| outro.mesma_fila(nome)))
}

impl<S: SistemaDeFilas> Filas<S> {
    pub fn novo(sistema: S, pastas: Pastas) -> Self {
        Self { sistema, pastas }
    }

    pub fn pastas(&self) -> &Pastas {
        &self.pastas
    }

    /// Garante a fila e guarda o modo. Repetir com o mesmo nome só atualiza o modo. Uma fila com o
    /// nome e outra configuração é recusada sem nenhuma mudança.
    pub fn criar(&self, curto: &str, modo: ModoFisico) -> Result<FilaVirtual, ErroFilas> {
        let nome = NomeVirtual::novo(curto)?;
        let porta = self.pastas.porta(&nome)?;
        // O registro vem primeiro: arquivo ruim ou cheio impede qualquer mudança no sistema.
        let caminho = self.pastas.registro();
        let mut registro = Registro::carregar(&caminho)?;
        if !registro.cabe(&nome) {
            return Err(ErroFilas::SemEspaco);
        }
        let filas = self.sistema.filas()?;
        match procurar(&filas, &nome) {
            Some(fila) if !e_do_simulador(fila, &porta) => {
                return Err(ErroFilas::FilaAlheia {
                    fila: fila.nome.clone(),
                    driver: fila.driver.clone(),
                    porta: fila.porta.clone(),
                })
            }
            Some(_) => {}
            None => self.criar_no_sistema(&nome, &porta)?,
        }
        registro.definir(nome.clone(), modo)?;
        registro.salvar(&caminho)?;
        Ok(FilaVirtual {
            nome,
            porta,
            modo: Some(modo),
            situacao: Situacao::Pronta,
        })
    }

    fn criar_no_sistema(&self, nome: &NomeVirtual, porta: &str) -> Result<(), ErroFilas> {
        let pasta = self.pastas.pasta_da_porta(nome);
        fs::create_dir_all(&pasta).map_err(|erro| ErroFilas::PastaDaPorta(pasta, erro))?;
        let porta_nova = !self.sistema.porta_existe(porta)?;
        if porta_nova {
            self.sistema.criar_porta(porta)?;
        }
        if let Err(erro) = self
            .sistema
            .criar_fila(&nome.fila(), DRIVER_DAS_FILAS, porta)
        {
            // Sem fila, a porta recém-criada não serve a ninguém.
            if porta_nova {
                let _ = self.sistema.remover_porta(porta);
            }
            return Err(erro.into());
        }
        Ok(())
    }

    /// Filas do registro, mais as que têm o prefixo no sistema. Outras filas não aparecem.
    pub fn listar(&self) -> Result<Vec<FilaVirtual>, ErroFilas> {
        let registro = Registro::carregar(&self.pastas.registro())?;
        let filas = self.sistema.filas()?;
        let mut lista = Vec::new();
        for entrada in registro.entradas() {
            let porta = self.pastas.porta(&entrada.nome)?;
            let situacao = match procurar(&filas, &entrada.nome) {
                None => Situacao::AusenteDoSistema,
                Some(fila) if e_do_simulador(fila, &porta) => Situacao::Pronta,
                Some(_) => Situacao::Diferente,
            };
            lista.push(FilaVirtual {
                nome: entrada.nome.clone(),
                porta,
                modo: Some(entrada.modo),
                situacao,
            });
        }
        for fila in &filas {
            let Ok(nome) = NomeVirtual::da_fila(&fila.nome) else {
                continue;
            };
            if registro.obter(&nome).is_some() {
                continue;
            }
            let porta = self.pastas.porta(&nome)?;
            let situacao = if e_do_simulador(fila, &porta) {
                Situacao::SemRegistro
            } else {
                Situacao::Diferente
            };
            lista.push(FilaVirtual {
                nome,
                porta,
                modo: None,
                situacao,
            });
        }
        Ok(lista)
    }

    /// Remove a fila, a porta dela e o arquivo de saída, e tira do registro. O histórico fica.
    /// Repetir não dá erro. Uma fila com o nome e outra configuração é recusada.
    pub fn remover(&self, curto: &str) -> Result<Remocao, ErroFilas> {
        let nome = NomeVirtual::novo(curto)?;
        let porta = self.pastas.porta(&nome)?;
        let caminho = self.pastas.registro();
        let mut registro = Registro::carregar(&caminho)?;
        let filas = self.sistema.filas()?;
        let mut fila_removida = false;
        if let Some(fila) = procurar(&filas, &nome) {
            if !e_do_simulador(fila, &porta) {
                return Err(ErroFilas::FilaAlheia {
                    fila: fila.nome.clone(),
                    driver: fila.driver.clone(),
                    porta: fila.porta.clone(),
                });
            }
            self.sistema.remover_fila(&fila.nome)?;
            fila_removida = true;
        }
        // Mesmo que outra fila use a porta, ela não é apagada.
        let porta_em_uso = filas
            .iter()
            .any(|fila| !mesmo_nome(fila, &nome) && mesmo_texto(&fila.porta, &porta));
        let mut porta_removida = false;
        if !porta_em_uso && self.sistema.porta_existe(&porta)? {
            self.sistema.remover_porta(&porta)?;
            porta_removida = true;
        }
        self.limpar_pasta_da_porta(&nome);
        registro.retirar(&nome);
        registro.salvar(&caminho)?;
        Ok(Remocao {
            fila_removida,
            porta_removida,
        })
    }

    /// Só o arquivo que o sistema grava e as pastas vazias. Qualquer outra coisa ali fica: apagar
    /// pasta com conteúdo falha, e isso é o que se quer.
    fn limpar_pasta_da_porta(&self, nome: &NomeVirtual) {
        let pasta = self.pastas.pasta_da_porta(nome);
        let _ = fs::remove_file(pasta.join(ARQUIVO_DA_PORTA));
        let _ = fs::remove_dir(&pasta);
        if let Some(raiz) = pasta.parent() {
            let _ = fs::remove_dir(raiz);
        }
    }
}

fn mesmo_nome(fila: &FilaDoSistema, nome: &NomeVirtual) -> bool {
    NomeVirtual::da_fila(&fila.nome).is_ok_and(|outro| outro.mesma_fila(nome))
}

#[cfg(test)]
mod testes {
    use super::*;
    use crate::modo::{FaceDeSaida, Saida};
    use crate::testes_comuns::PastaTemporaria;
    use std::cell::RefCell;

    const MODO: ModoFisico = ModoFisico {
        saida: Saida::Gaveta,
        face: FaceDeSaida::ParaBaixo,
    };
    const OUTRO_MODO: ModoFisico = ModoFisico {
        saida: Saida::Traseira,
        face: FaceDeSaida::ParaCima,
    };

    /// Sistema em memória que anota cada mudança.
    #[derive(Default)]
    struct SistemaFalso {
        filas: RefCell<Vec<FilaDoSistema>>,
        portas: RefCell<Vec<String>>,
        mudancas: RefCell<Vec<String>>,
        negar: bool,
        falhar_fila: bool,
    }

    impl SistemaFalso {
        fn com_filas(filas: Vec<FilaDoSistema>) -> Self {
            Self {
                filas: RefCell::new(filas),
                ..Default::default()
            }
        }
    }

    impl SistemaDeFilas for &SistemaFalso {
        fn filas(&self) -> Result<Vec<FilaDoSistema>, ErroSistema> {
            Ok(self.filas.borrow().clone())
        }
        fn porta_existe(&self, porta: &str) -> Result<bool, ErroSistema> {
            Ok(self.portas.borrow().iter().any(|p| p == porta))
        }
        fn criar_porta(&self, porta: &str) -> Result<(), ErroSistema> {
            if self.negar {
                return Err(ErroSistema::PrecisaAdministrador);
            }
            self.mudancas.borrow_mut().push(format!("+porta {porta}"));
            self.portas.borrow_mut().push(porta.to_string());
            Ok(())
        }
        fn criar_fila(&self, fila: &str, driver: &str, porta: &str) -> Result<(), ErroSistema> {
            if self.falhar_fila {
                return Err(ErroSistema::Recusado {
                    codigo: "x".to_string(),
                    mensagem: "y".to_string(),
                });
            }
            self.mudancas.borrow_mut().push(format!("+fila {fila}"));
            self.filas.borrow_mut().push(FilaDoSistema {
                nome: fila.to_string(),
                driver: driver.to_string(),
                porta: porta.to_string(),
            });
            Ok(())
        }
        fn remover_fila(&self, fila: &str) -> Result<(), ErroSistema> {
            self.mudancas.borrow_mut().push(format!("-fila {fila}"));
            self.filas.borrow_mut().retain(|f| f.nome != fila);
            Ok(())
        }
        fn remover_porta(&self, porta: &str) -> Result<(), ErroSistema> {
            self.mudancas.borrow_mut().push(format!("-porta {porta}"));
            self.portas.borrow_mut().retain(|p| p != porta);
            Ok(())
        }
    }

    fn pastas(temporaria: &PastaTemporaria) -> Pastas {
        Pastas::novas(temporaria.caminho("config"), temporaria.caminho("portas")).unwrap()
    }

    #[test]
    fn criar_faz_porta_fila_registro_e_pasta_e_listar_mostra() {
        let temporaria = PastaTemporaria::nova();
        let sistema = SistemaFalso::default();
        let filas = Filas::novo(&sistema, pastas(&temporaria));
        let criada = filas.criar("Jato", MODO).unwrap();
        assert_eq!(criada.situacao, Situacao::Pronta);
        let mudancas = sistema.mudancas.borrow().clone();
        assert_eq!(mudancas.len(), 2);
        assert!(mudancas[0].starts_with("+porta "));
        assert_eq!(mudancas[1], "+fila PlotSim — Jato");
        assert_eq!(sistema.filas.borrow()[0].driver, DRIVER_DAS_FILAS);
        assert!(std::path::Path::new(&criada.porta)
            .parent()
            .unwrap()
            .is_dir());
        assert_eq!(filas.listar().unwrap(), vec![criada]);
    }

    #[test]
    fn criar_de_novo_nao_toca_no_sistema_e_atualiza_o_modo() {
        let temporaria = PastaTemporaria::nova();
        let sistema = SistemaFalso::default();
        let filas = Filas::novo(&sistema, pastas(&temporaria));
        filas.criar("Jato", MODO).unwrap();
        let antes = sistema.mudancas.borrow().len();
        filas.criar("JATO", OUTRO_MODO).unwrap();
        assert_eq!(sistema.mudancas.borrow().len(), antes);
        let lista = filas.listar().unwrap();
        assert_eq!(lista.len(), 1);
        assert_eq!(lista[0].modo, Some(OUTRO_MODO));
    }

    #[test]
    fn fila_com_o_nome_e_outra_configuracao_nunca_e_alterada() {
        let temporaria = PastaTemporaria::nova();
        let sistema = SistemaFalso::com_filas(vec![FilaDoSistema {
            nome: "PlotSim — Jato".to_string(),
            driver: "Driver Real".to_string(),
            porta: "USB001".to_string(),
        }]);
        let filas = Filas::novo(&sistema, pastas(&temporaria));
        assert!(matches!(
            filas.criar("Jato", MODO),
            Err(ErroFilas::FilaAlheia { .. })
        ));
        assert!(matches!(
            filas.remover("Jato"),
            Err(ErroFilas::FilaAlheia { .. })
        ));
        assert!(sistema.mudancas.borrow().is_empty());
        assert_eq!(filas.listar().unwrap()[0].situacao, Situacao::Diferente);
    }

    #[test]
    fn outras_filas_nao_aparecem_e_nao_sao_removidas() {
        let temporaria = PastaTemporaria::nova();
        let alheia = FilaDoSistema {
            nome: "Microsoft Print to PDF".to_string(),
            driver: DRIVER_DAS_FILAS.to_string(),
            porta: "PORTPROMPT:".to_string(),
        };
        let sistema = SistemaFalso::com_filas(vec![alheia.clone()]);
        let filas = Filas::novo(&sistema, pastas(&temporaria));
        assert!(filas.listar().unwrap().is_empty());
        filas.remover("Microsoft Print to PDF").unwrap();
        assert!(sistema.mudancas.borrow().is_empty());
        assert_eq!(sistema.filas.borrow().clone(), vec![alheia]);
    }

    #[test]
    fn nome_hostil_para_antes_do_sistema() {
        let temporaria = PastaTemporaria::nova();
        let sistema = SistemaFalso::default();
        let filas = Filas::novo(&sistema, pastas(&temporaria));
        for hostil in ["a\"; Remove-Printer *", "..\\x", "*", ""] {
            assert!(matches!(filas.criar(hostil, MODO), Err(ErroFilas::Nome(_))));
            assert!(matches!(filas.remover(hostil), Err(ErroFilas::Nome(_))));
        }
        assert!(sistema.mudancas.borrow().is_empty());
        assert!(!temporaria.caminho("portas").exists());
    }

    #[test]
    fn negacao_do_windows_vira_erro_proprio_e_nada_fica_registrado() {
        let temporaria = PastaTemporaria::nova();
        let sistema = SistemaFalso {
            negar: true,
            ..Default::default()
        };
        let filas = Filas::novo(&sistema, pastas(&temporaria));
        assert!(matches!(
            filas.criar("Jato", MODO),
            Err(ErroFilas::Sistema(ErroSistema::PrecisaAdministrador))
        ));
        assert!(!temporaria.caminho("config/filas.toml").exists());
    }

    #[test]
    fn falha_ao_criar_a_fila_desfaz_a_porta_recem_criada() {
        let temporaria = PastaTemporaria::nova();
        let sistema = SistemaFalso {
            falhar_fila: true,
            ..Default::default()
        };
        let filas = Filas::novo(&sistema, pastas(&temporaria));
        assert!(filas.criar("Jato", MODO).is_err());
        assert!(sistema.portas.borrow().is_empty());
        assert!(!temporaria.caminho("config/filas.toml").exists());
    }

    #[test]
    fn remover_desfaz_tudo_mantem_o_historico_e_pode_repetir() {
        let temporaria = PastaTemporaria::nova();
        let sistema = SistemaFalso::default();
        let filas = Filas::novo(&sistema, pastas(&temporaria));
        let criada = filas.criar("Jato", MODO).unwrap();
        let nome = NomeVirtual::novo("Jato").unwrap();
        let historico = filas.pastas().historico(&nome);
        fs::create_dir_all(&historico).unwrap();
        fs::write(historico.join("1.pdf"), b"x").unwrap();
        fs::write(&criada.porta, b"%PDF").unwrap();
        assert_eq!(
            filas.remover("Jato").unwrap(),
            Remocao {
                fila_removida: true,
                porta_removida: true,
            }
        );
        assert!(sistema.filas.borrow().is_empty());
        assert!(sistema.portas.borrow().is_empty());
        assert!(!std::path::Path::new(&criada.porta)
            .parent()
            .unwrap()
            .exists());
        assert!(historico.join("1.pdf").exists());
        assert!(filas.listar().unwrap().is_empty());
        assert_eq!(
            filas.remover("Jato").unwrap(),
            Remocao {
                fila_removida: false,
                porta_removida: false,
            }
        );
    }

    #[test]
    fn arquivo_estranho_na_pasta_da_porta_nao_e_apagado() {
        let temporaria = PastaTemporaria::nova();
        let sistema = SistemaFalso::default();
        let filas = Filas::novo(&sistema, pastas(&temporaria));
        let criada = filas.criar("Jato", MODO).unwrap();
        let pasta = std::path::Path::new(&criada.porta).parent().unwrap();
        fs::write(pasta.join("meu.txt"), b"x").unwrap();
        filas.remover("Jato").unwrap();
        assert!(pasta.join("meu.txt").exists());
    }

    #[test]
    fn raiz_das_portas_vazia_some_com_a_ultima_fila_e_cheia_fica() {
        let temporaria = PastaTemporaria::nova();
        let sistema = SistemaFalso::default();
        let filas = Filas::novo(&sistema, pastas(&temporaria));
        filas.criar("Um", MODO).unwrap();
        filas.criar("Dois", MODO).unwrap();
        filas.remover("Um").unwrap();
        assert!(temporaria.caminho("portas").is_dir());
        filas.remover("Dois").unwrap();
        assert!(!temporaria.caminho("portas").exists());
    }

    #[test]
    fn porta_usada_por_outra_fila_nao_e_removida() {
        let temporaria = PastaTemporaria::nova();
        let sistema = SistemaFalso::default();
        let filas = Filas::novo(&sistema, pastas(&temporaria));
        let criada = filas.criar("Jato", MODO).unwrap();
        sistema.filas.borrow_mut().push(FilaDoSistema {
            nome: "Outra fila".to_string(),
            driver: "Outro".to_string(),
            porta: criada.porta.clone(),
        });
        let remocao = filas.remover("Jato").unwrap();
        assert!(remocao.fila_removida);
        assert!(!remocao.porta_removida);
        assert_eq!(sistema.portas.borrow().clone(), vec![criada.porta]);
        assert_eq!(sistema.filas.borrow()[0].nome, "Outra fila");
    }

    #[test]
    fn registro_ruim_impede_mudar_o_sistema() {
        let temporaria = PastaTemporaria::nova();
        fs::create_dir_all(temporaria.caminho("config")).unwrap();
        fs::write(temporaria.caminho("config/filas.toml"), "ruim [").unwrap();
        let sistema = SistemaFalso::default();
        let filas = Filas::novo(&sistema, pastas(&temporaria));
        assert!(matches!(
            filas.criar("Jato", MODO),
            Err(ErroFilas::Registro(_))
        ));
        assert!(matches!(filas.remover("Jato"), Err(ErroFilas::Registro(_))));
        assert!(sistema.mudancas.borrow().is_empty());
    }

    #[test]
    fn fila_com_prefixo_sem_registro_aparece_sem_modo_e_pode_ser_removida() {
        let temporaria = PastaTemporaria::nova();
        let caminhos = pastas(&temporaria);
        let porta = caminhos
            .porta(&NomeVirtual::novo("Sobra").unwrap())
            .unwrap();
        let sistema = SistemaFalso::com_filas(vec![FilaDoSistema {
            nome: "PlotSim — Sobra".to_string(),
            driver: DRIVER_DAS_FILAS.to_string(),
            porta,
        }]);
        let filas = Filas::novo(&sistema, caminhos);
        let lista = filas.listar().unwrap();
        assert_eq!(lista[0].situacao, Situacao::SemRegistro);
        assert_eq!(lista[0].modo, None);
        assert!(filas.remover("Sobra").unwrap().fila_removida);
    }

    #[test]
    fn fila_do_registro_que_sumiu_aparece_como_ausente() {
        let temporaria = PastaTemporaria::nova();
        let sistema = SistemaFalso::default();
        let filas = Filas::novo(&sistema, pastas(&temporaria));
        filas.criar("Jato", MODO).unwrap();
        sistema.filas.borrow_mut().clear();
        assert_eq!(
            filas.listar().unwrap()[0].situacao,
            Situacao::AusenteDoSistema
        );
    }
}
