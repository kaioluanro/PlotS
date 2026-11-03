//! Conferência do maço final: ordem, orientação e faces, com o motivo em dado e em texto.

use std::collections::HashMap;
use std::fmt;

use crate::geometria::Vetor;
use crate::impressora::{Face, Folha, Lado, Operacao, Pilha};

/// Como a pessoa pega o maço para ler.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Leitura {
    ComoSaiu,
    /// Vira o maço como página de livro antes de ler.
    VirandoOMaco,
}

/// Uma folha como a pessoa a vê: com a frente voltada para ela.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct FolhaLida {
    /// Páginas gravadas na face de cima. Normalmente zero ou uma.
    pub frente: Vec<u32>,
    pub verso: Vec<u32>,
    /// Topo da frente e do verso na mesma aresta da folha. `None` quando falta alguma das faces.
    pub topos_na_mesma_aresta: Option<bool>,
    /// O topo aponta para a mesma aresta do maço que o da primeira folha com imagem.
    pub alinhada_com_a_primeira: Option<bool>,
    /// Alguma imagem está deitada em relação à outra face: guarda a página.
    pub pagina_deitada: Option<u32>,
}

/// O maço lido de cima para baixo, independente da máquina que o produziu.
#[derive(Clone, Debug, PartialEq, Eq, Default)]
pub struct MacoLido {
    pub folhas: Vec<FolhaLida>,
}

/// Motivo de reprovação.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Motivo {
    FolhaAMenos {
        esperadas: u32,
        encontradas: u32,
    },
    FolhaAMais {
        esperadas: u32,
        encontradas: u32,
    },
    VersoDeCabecaParaBaixo {
        folha: u32,
    },
    /// O topo de uma imagem aponta para uma aresta longa: página deitada na folha.
    PaginaGirada {
        folha: u32,
        pagina: u32,
    },
    /// A folha toda está invertida em relação à primeira do maço.
    FolhaDeCabecaParaBaixo {
        folha: u32,
    },
    FrenteTrocada {
        folha: u32,
        com_folha: u32,
        pagina: u32,
    },
    VersoTrocado {
        folha: u32,
        com_folha: u32,
        pagina: u32,
    },
    /// As duas páginas certas estão na folha, com as faces trocadas: o par está voltado ao contrário.
    FolhaAoContrario {
        folha: u32,
    },
    PaginaEstranha {
        folha: u32,
        pagina: u32,
    },
    FaltaPagina {
        folha: u32,
        pagina: u32,
    },
    ImpressaoSobreImpressao {
        folha: u32,
    },
}

/// Idiomas da interface.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Idioma {
    PtBr,
    En,
}

impl Motivo {
    /// O motivo em texto, no idioma pedido.
    pub fn texto(self, idioma: Idioma) -> String {
        match idioma {
            Idioma::PtBr => self.em_portugues(),
            Idioma::En => self.em_ingles(),
        }
    }

    fn em_portugues(self) -> String {
        match self {
            Motivo::FolhaAMenos { .. } => "folha a menos".to_owned(),
            Motivo::FolhaAMais { .. } => "folha a mais".to_owned(),
            Motivo::VersoDeCabecaParaBaixo { folha } => {
                format!("verso de cabeça para baixo na folha {folha}")
            }
            Motivo::PaginaGirada { folha, pagina } => {
                format!("página {pagina} girada na folha {folha}")
            }
            Motivo::FolhaDeCabecaParaBaixo { folha } => {
                format!("folha {folha} de cabeça para baixo no maço")
            }
            Motivo::FrenteTrocada {
                folha, com_folha, ..
            } => format!("frente da folha {folha} trocada com a folha {com_folha}"),
            Motivo::VersoTrocado { com_folha, .. } => {
                format!("verso trocado com a folha {com_folha}")
            }
            Motivo::FolhaAoContrario { folha } => {
                format!("folha {folha} com frente e verso invertidos")
            }
            Motivo::PaginaEstranha { folha, pagina } => {
                format!("página {pagina} não pertence ao documento, na folha {folha}")
            }
            Motivo::FaltaPagina { folha, pagina } => {
                format!("falta a página {pagina} na folha {folha}")
            }
            Motivo::ImpressaoSobreImpressao { folha } => {
                format!("impressão sobre impressão na folha {folha}")
            }
        }
    }

    fn em_ingles(self) -> String {
        match self {
            Motivo::FolhaAMenos { .. } => "one sheet short".to_owned(),
            Motivo::FolhaAMais { .. } => "one sheet too many".to_owned(),
            Motivo::VersoDeCabecaParaBaixo { folha } => {
                format!("back upside down on sheet {folha}")
            }
            Motivo::PaginaGirada { folha, pagina } => {
                format!("page {pagina} rotated on sheet {folha}")
            }
            Motivo::FolhaDeCabecaParaBaixo { folha } => {
                format!("sheet {folha} upside down in the stack")
            }
            Motivo::FrenteTrocada {
                folha, com_folha, ..
            } => format!("front of sheet {folha} swapped with sheet {com_folha}"),
            Motivo::VersoTrocado { com_folha, .. } => {
                format!("back swapped with sheet {com_folha}")
            }
            Motivo::FolhaAoContrario { folha } => {
                format!("sheet {folha} has front and back reversed")
            }
            Motivo::PaginaEstranha { folha, pagina } => {
                format!("page {pagina} is not in the document, on sheet {folha}")
            }
            Motivo::FaltaPagina { folha, pagina } => {
                format!("page {pagina} missing on sheet {folha}")
            }
            Motivo::ImpressaoSobreImpressao { folha } => {
                format!("printed over a printed side on sheet {folha}")
            }
        }
    }
}

impl fmt::Display for Motivo {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.texto(Idioma::PtBr))
    }
}

/// Lê o maço na pose final. Não altera a pilha recebida.
pub fn ler_maco(pilha: &Pilha, leitura: Leitura) -> MacoLido {
    let mut maco = pilha.clone();
    if leitura == Leitura::VirandoOMaco {
        maco.aplicar(Operacao::VirarPelaArestaLonga);
    }
    let mut referencia: Option<Vetor> = None;
    let folhas = maco
        .folhas()
        .iter()
        .rev()
        .map(|folha| ler_folha(folha, &mut referencia))
        .collect();
    MacoLido { folhas }
}

fn ler_folha(folha: &Folha, referencia: &mut Option<Vetor>) -> FolhaLida {
    let frente = folha.face_para(Lado::Cima);
    let verso = match frente {
        Face::A => Face::B,
        Face::B => Face::A,
    };
    let imp_frente = folha.impressoes(frente);
    let imp_verso = folha.impressoes(verso);
    let topos_na_mesma_aresta = match (imp_frente.first(), imp_verso.first()) {
        (Some(a), Some(b)) => Some(a.topo == b.topo),
        _ => None,
    };
    let imagem = imp_frente.first().or(imp_verso.first());
    let topo = imagem.map(|i| folha.topo_no_mundo(*i));
    let alinhada_com_a_primeira = match (topo, *referencia) {
        (Some(t), Some(r)) => Some(t == r),
        (Some(t), None) => {
            *referencia = Some(t);
            Some(true)
        }
        (None, _) => None,
    };
    // A primeira folha com imagem define o eixo do maço; uma imagem que não está nesse eixo, nem
    // na frente nem no verso, está deitada.
    let eixo = referencia.unwrap_or(Vetor::Y);
    let pagina_deitada = imp_frente
        .iter()
        .chain(imp_verso.iter())
        .find(|i| folha.topo_no_mundo(**i).escalar(eixo) == 0)
        .map(|i| i.pagina);
    FolhaLida {
        frente: imp_frente.iter().map(|i| i.pagina).collect(),
        verso: imp_verso.iter().map(|i| i.pagina).collect(),
        topos_na_mesma_aresta,
        alinhada_com_a_primeira,
        pagina_deitada,
    }
}

/// Quantidade de folhas de um documento: uma folha para cada duas páginas, arredondando para cima.
pub const fn folhas_do_documento(total_paginas: u32) -> u32 {
    total_paginas.div_ceil(2)
}

fn u32_saturado(valor: usize) -> u32 {
    u32::try_from(valor).unwrap_or(u32::MAX)
}

/// Compara o maço lido com o que o documento exige: cada folha com a página ímpar `n` e `n + 1`
/// (ou em branco no fim de total ímpar), maço em 1, 2, 3… e o topo das duas faces na mesma aresta.
pub fn conferir(maco: &MacoLido, total_paginas: u32) -> Vec<Motivo> {
    let mut motivos = Vec::new();
    let esperadas = folhas_do_documento(total_paginas);
    let encontradas = u32_saturado(maco.folhas.len());
    if encontradas < esperadas {
        motivos.push(Motivo::FolhaAMenos {
            esperadas,
            encontradas,
        });
    } else if encontradas > esperadas {
        motivos.push(Motivo::FolhaAMais {
            esperadas,
            encontradas,
        });
    }
    let onde_esta = onde_cada_pagina_esta(maco);
    for (posicao, folha) in maco.folhas.iter().enumerate() {
        let numero = u32_saturado(posicao).saturating_add(1);
        conferir_folha(numero, folha, total_paginas, &onde_esta, &mut motivos);
    }
    motivos
}

fn onde_cada_pagina_esta(maco: &MacoLido) -> HashMap<u32, u32> {
    let mut onde = HashMap::new();
    for (posicao, folha) in maco.folhas.iter().enumerate() {
        let numero = u32_saturado(posicao).saturating_add(1);
        for pagina in folha.frente.iter().chain(folha.verso.iter()) {
            onde.entry(*pagina).or_insert(numero);
        }
    }
    onde
}

fn conferir_folha(
    numero: u32,
    folha: &FolhaLida,
    total: u32,
    onde_esta: &HashMap<u32, u32>,
    motivos: &mut Vec<Motivo>,
) {
    if folha.frente.len() > 1 || folha.verso.len() > 1 {
        motivos.push(Motivo::ImpressaoSobreImpressao { folha: numero });
    }
    let impar = numero.saturating_mul(2).saturating_sub(1);
    let par = numero.saturating_mul(2);
    let pagina_da_frente = (impar <= total).then_some(impar);
    let pagina_do_verso = (par <= total).then_some(par);
    let frente = folha.frente.first().copied();
    let verso = folha.verso.first().copied();

    if frente.is_some() && frente == pagina_do_verso && verso == pagina_da_frente {
        motivos.push(Motivo::FolhaAoContrario { folha: numero });
    } else {
        let contexto = Contexto {
            numero,
            total,
            onde_esta,
        };
        contexto.conferir_face(frente, pagina_da_frente, true, motivos);
        contexto.conferir_face(verso, pagina_do_verso, false, motivos);
    }

    if folha.topos_na_mesma_aresta == Some(false) {
        motivos.push(Motivo::VersoDeCabecaParaBaixo { folha: numero });
    }
    if folha.alinhada_com_a_primeira == Some(false) {
        motivos.push(Motivo::FolhaDeCabecaParaBaixo { folha: numero });
    }
    if let Some(pagina) = folha.pagina_deitada {
        motivos.push(Motivo::PaginaGirada {
            folha: numero,
            pagina,
        });
    }
}

struct Contexto<'a> {
    numero: u32,
    total: u32,
    onde_esta: &'a HashMap<u32, u32>,
}

impl Contexto<'_> {
    fn conferir_face(
        &self,
        encontrada: Option<u32>,
        esperada: Option<u32>,
        e_frente: bool,
        motivos: &mut Vec<Motivo>,
    ) {
        let folha = self.numero;
        if encontrada == esperada {
            return;
        }
        let Some(pagina) = encontrada else {
            if let Some(pagina) = esperada {
                motivos.push(Motivo::FaltaPagina { folha, pagina });
            }
            return;
        };
        if pagina == 0 || pagina > self.total {
            motivos.push(Motivo::PaginaEstranha { folha, pagina });
            return;
        }
        // A folha certa da página encontrada é a que ela deveria ocupar, sempre que for outra.
        let dona = pagina.div_ceil(2);
        let com_folha = if dona != folha {
            dona
        } else {
            self.onde_esta.get(&pagina).copied().unwrap_or(folha)
        };
        motivos.push(if e_frente {
            Motivo::FrenteTrocada {
                folha,
                com_folha,
                pagina,
            }
        } else {
            Motivo::VersoTrocado {
                folha,
                com_folha,
                pagina,
            }
        });
    }
}
