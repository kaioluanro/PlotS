//! Folhas, pilhas e a impressora: modo, caminho do papel e operações físicas sobre o maço.

use crate::geometria::{Giro, Rotacao, Vetor};

/// Limite de folhas em uma pilha. Número vindo de fora não pode pedir memória sem fim.
pub const LIMITE_FOLHAS: usize = 200_000;

/// Um dos dois sentidos verticais.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Lado {
    Cima,
    Baixo,
}

impl Lado {
    const fn normal(self) -> Vetor {
        match self {
            Lado::Cima => Vetor::Z,
            Lado::Baixo => Vetor::novo(0, 0, -1),
        }
    }
}

/// Face física da folha.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Face {
    A,
    B,
}

/// Por onde a pilha entra.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Entrada {
    Traseira,
    Gaveta,
    Manual,
}

/// Onde o topo da imagem fica em relação ao andar da folha, sem nenhum giro no envio.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum TopoDaImagem {
    /// O topo da imagem fica na aresta que entra primeiro.
    NaArestaQueEntra,
    /// O topo da imagem fica na aresta que entra por último.
    NaArestaQueSai,
}

/// Como uma impressora trata o papel.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct ModoImpressora {
    pub entrada: Entrada,
    /// A folha puxada da entrada é a de cima ou a de baixo.
    pub puxa: Lado,
    /// A face que recebe a tinta é a voltada para cima ou para baixo, na entrada.
    pub imprime_na_face_para: Lado,
    /// A face impressa por último sai voltada para cima ou para baixo.
    pub sai_com_face_para: Lado,
    /// Na saída, a folha nova fica por cima ou por baixo das anteriores.
    pub cai: Lado,
    pub topo_da_imagem: TopoDaImagem,
}

impl ModoImpressora {
    /// Quando as faces de entrada e de saída diferem, o papel faz uma curva em U: a aresta que
    /// entrou primeiro passa a apontar para o lado oposto e a face de cima passa para baixo.
    /// Quando são iguais, o caminho é reto e a folha só se desloca.
    const fn caminho(self) -> Rotacao {
        if matches!(
            (self.imprime_na_face_para, self.sai_com_face_para),
            (Lado::Cima, Lado::Cima) | (Lado::Baixo, Lado::Baixo)
        ) {
            Rotacao::IDENTIDADE
        } else {
            Rotacao::MEIA_VOLTA_X
        }
    }
}

/// Uma imagem gravada, com a direção do seu topo no material da folha.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct Impressao {
    pub pagina: u32,
    pub topo: Vetor,
}

/// Página enviada à impressora.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct Envio {
    /// `None` é folha que passa em branco.
    pub pagina: Option<u32>,
    /// Giro pedido no envio, somado a qualquer giro que a página já tenha.
    pub giro: Giro,
}

impl Envio {
    pub const fn pagina(numero: u32) -> Self {
        Self {
            pagina: Some(numero),
            giro: Giro::Nenhum,
        }
    }

    pub const fn branco() -> Self {
        Self {
            pagina: None,
            giro: Giro::Nenhum,
        }
    }

    pub const fn girada(numero: u32, giro: Giro) -> Self {
        Self {
            pagina: Some(numero),
            giro,
        }
    }
}

/// Uma folha de papel e a sua pose no mundo.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Folha {
    pose: Rotacao,
    face_a: Vec<Impressao>,
    face_b: Vec<Impressao>,
}

impl Folha {
    fn nova() -> Self {
        Self {
            pose: Rotacao::IDENTIDADE,
            face_a: Vec::new(),
            face_b: Vec::new(),
        }
    }

    /// Face voltada para o lado pedido, na pose atual.
    pub fn face_para(&self, lado: Lado) -> Face {
        if self.pose.normal_da_face_a().escalar(lado.normal()) > 0 {
            Face::A
        } else {
            Face::B
        }
    }

    /// Tudo o que foi gravado em uma face. Mais de um item é impressão sobre impressão.
    pub fn impressoes(&self, face: Face) -> &[Impressao] {
        match face {
            Face::A => &self.face_a,
            Face::B => &self.face_b,
        }
    }

    /// Direção do topo de uma imagem no mundo, na pose atual da folha.
    pub fn topo_no_mundo(&self, impressao: Impressao) -> Vetor {
        self.pose.aplicar(impressao.topo)
    }

    fn gravar(&mut self, lado: Lado, pagina: u32, topo_no_mundo: Vetor) {
        let impressao = Impressao {
            pagina,
            topo: self.pose.desfazer(topo_no_mundo),
        };
        match self.face_para(lado) {
            Face::A => self.face_a.push(impressao),
            Face::B => self.face_b.push(impressao),
        }
    }

    fn mover(&mut self, rotacao: Rotacao) {
        self.pose = self.pose.seguida_de(rotacao);
    }

    fn alguma_impressao(&self) -> Option<(Face, Impressao)> {
        if let Some(i) = self.face_a.last() {
            return Some((Face::A, *i));
        }
        self.face_b.last().map(|i| (Face::B, *i))
    }
}

/// Operação física feita com as mãos sobre a pilha inteira.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Operacao {
    /// Vira a pilha por uma aresta curta, como bloco de notas. Quem estava em cima fica embaixo.
    VirarPelaArestaCurta,
    /// Vira a pilha por uma aresta longa, como página de livro. Quem estava em cima fica embaixo.
    VirarPelaArestaLonga,
    /// Gira a pilha meia volta sobre a mesa. A ordem não muda.
    Girar180,
    /// Passa as folhas uma a uma para outro monte, sem virar nenhuma. A ordem se inverte.
    InverterFolhaAFolha,
}

/// Pilha ordenada: o primeiro elemento é a folha de baixo.
#[derive(Clone, Debug, PartialEq, Eq, Default)]
pub struct Pilha {
    folhas: Vec<Folha>,
}

impl Pilha {
    pub fn vazia() -> Self {
        Self::default()
    }

    /// Papel em branco, limitado ao teto de folhas.
    pub fn de_papel(quantidade: usize) -> Self {
        let n = quantidade.min(LIMITE_FOLHAS);
        Self {
            folhas: (0..n).map(|_| Folha::nova()).collect(),
        }
    }

    pub fn len(&self) -> usize {
        self.folhas.len()
    }

    pub fn is_empty(&self) -> bool {
        self.folhas.is_empty()
    }

    /// Folhas de baixo para cima.
    pub fn folhas(&self) -> &[Folha] {
        &self.folhas
    }

    /// Retira uma folha, contando de cima. Fora da pilha, não faz nada e devolve `None`.
    pub fn retirar_de_cima(&mut self, posicao: usize) -> Option<Folha> {
        let indice = self.folhas.len().checked_sub(1)?.checked_sub(posicao)?;
        Some(self.folhas.remove(indice))
    }

    /// Coloca uma pilha por cima desta, sem mudar a pose de nada.
    pub fn acrescentar_por_cima(&mut self, outra: Pilha) {
        self.folhas.extend(outra.folhas);
    }

    /// Aplica uma operação. A ordem das folhas segue da própria rotação: se o eixo vertical se
    /// inverte, quem estava em cima passa para baixo.
    pub fn aplicar(&mut self, operacao: Operacao) {
        let rotacao = match operacao {
            Operacao::VirarPelaArestaCurta => Rotacao::MEIA_VOLTA_X,
            Operacao::VirarPelaArestaLonga => Rotacao::MEIA_VOLTA_Y,
            Operacao::Girar180 => Rotacao::MEIA_VOLTA_Z,
            Operacao::InverterFolhaAFolha => {
                self.folhas.reverse();
                return;
            }
        };
        for folha in &mut self.folhas {
            folha.mover(rotacao);
        }
        if rotacao.aplicar(Vetor::Z).z < 0 {
            self.folhas.reverse();
        }
    }

    pub fn aplicar_todas(&mut self, operacoes: &[Operacao]) {
        for operacao in operacoes {
            self.aplicar(*operacao);
        }
    }

    fn puxar(&mut self, lado: Lado) -> Option<Folha> {
        match lado {
            Lado::Cima => self.folhas.pop(),
            Lado::Baixo => {
                if self.folhas.is_empty() {
                    None
                } else {
                    Some(self.folhas.remove(0))
                }
            }
        }
    }

    fn receber(&mut self, folha: Folha, lado: Lado) {
        match lado {
            Lado::Cima => self.folhas.push(folha),
            Lado::Baixo => self.folhas.insert(0, folha),
        }
    }

    /// O sentido em que aponta o topo da maior parte das imagens. No empate, vale `empate`.
    fn sentido_da_maioria(&self, empate: Vetor) -> Vetor {
        let mut saldo = 0i64;
        for folha in &self.folhas {
            if let Some((_, impressao)) = folha.alguma_impressao() {
                saldo = saldo.saturating_add(i64::from(folha.topo_no_mundo(impressao).y));
            }
        }
        match saldo {
            0 => empate,
            s if s > 0 => Vetor::Y,
            _ => Vetor::Y.oposto(),
        }
    }

    /// Primeira impressão encontrada a partir de cima, com a folha que a contém.
    fn referencia(&self) -> Option<(&Folha, Face, Impressao)> {
        self.folhas
            .iter()
            .rev()
            .find_map(|f| f.alguma_impressao().map(|(face, i)| (f, face, i)))
    }
}

/// Face impressa como a instrução a descreve.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum FaceNaEntrada {
    ParaFrente,
    ParaTras,
    ParaCima,
    ParaBaixo,
}

/// Borda da página impressa que entra primeiro na máquina.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum BordaQueEntra {
    Topo,
    Base,
}

/// A instrução de recolocação como dado.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct Gesto {
    pub entrada: Entrada,
    pub face_impressa: FaceNaEntrada,
    pub borda_que_entra: BordaQueEntra,
    pub inverter_ordem: bool,
}

/// O gesto não pode ser cumprido nesta máquina.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum ErroGesto {
    /// A instrução manda para uma entrada que a máquina não usa.
    EntradaDiferente {
        pedida: Entrada,
        da_maquina: Entrada,
    },
    /// Na traseira a folha fica de pé; na gaveta, deitada. A face pedida não existe nessa entrada.
    FaceSemSentido {
        entrada: Entrada,
        face: FaceNaEntrada,
    },
}

impl Gesto {
    /// A face da instrução, no referencial deitado do modelo. Na entrada traseira, a folha se
    /// apoia inclinada: a face que olha para a frente da máquina é a de cima.
    fn lado(self) -> Result<Lado, ErroGesto> {
        match (self.entrada, self.face_impressa) {
            (Entrada::Traseira, FaceNaEntrada::ParaFrente) => Ok(Lado::Cima),
            (Entrada::Traseira, FaceNaEntrada::ParaTras) => Ok(Lado::Baixo),
            (Entrada::Gaveta | Entrada::Manual, FaceNaEntrada::ParaCima) => Ok(Lado::Cima),
            (Entrada::Gaveta | Entrada::Manual, FaceNaEntrada::ParaBaixo) => Ok(Lado::Baixo),
            (entrada, face) => Err(ErroGesto::FaceSemSentido { entrada, face }),
        }
    }

    /// Traduz a instrução nas operações físicas que a cumprem sobre esta pilha: vira, se a face
    /// impressa não estiver do lado pedido; gira, se a borda pedida não estiver na frente; e só
    /// depois inverte a ordem. Decide olhando a posição real das folhas, nunca o nome do modo. A
    /// pessoa olha o maço, não uma folha: uma página girada não muda o sentido dos outros.
    pub fn operacoes(self, pilha: &Pilha) -> Result<Vec<Operacao>, ErroGesto> {
        let lado = self.lado()?;
        let mut ops = Vec::new();
        if let Some((folha, face, impressao)) = pilha.referencia() {
            let mut topo = pilha.sentido_da_maioria(folha.topo_no_mundo(impressao));
            if folha.face_para(lado) != face {
                ops.push(Operacao::VirarPelaArestaCurta);
                topo = Rotacao::MEIA_VOLTA_X.aplicar(topo);
            }
            let aresta_da_frente = match self.borda_que_entra {
                BordaQueEntra::Topo => topo,
                BordaQueEntra::Base => topo.oposto(),
            };
            if aresta_da_frente == Vetor::Y.oposto() {
                ops.push(Operacao::Girar180);
            }
        }
        if self.inverter_ordem {
            ops.push(Operacao::InverterFolhaAFolha);
        }
        Ok(ops)
    }
}

/// O que sobrou de uma passada.
#[derive(Clone, Debug, PartialEq, Eq, Default)]
pub struct ResumoDaPassada {
    /// Páginas que não acharam folha na entrada.
    pub sem_folha: Vec<Envio>,
}

/// Regra da união da impressão automática nas duas faces.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Uniao {
    /// Os dois topos caem na mesma aresta da folha.
    PelaArestaLonga,
    /// Os dois topos caem em arestas opostas da folha.
    PelaArestaCurta,
}

/// Impressora com entrada e saída físicas.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Impressora {
    modo: ModoImpressora,
    entrada: Pilha,
    saida: Pilha,
}

impl Impressora {
    pub fn nova(modo: ModoImpressora, papel: usize) -> Self {
        Self {
            modo,
            entrada: Pilha::de_papel(papel),
            saida: Pilha::vazia(),
        }
    }

    pub const fn modo(&self) -> ModoImpressora {
        self.modo
    }

    pub const fn entrada(&self) -> &Pilha {
        &self.entrada
    }

    pub const fn saida(&self) -> &Pilha {
        &self.saida
    }

    pub fn saida_mut(&mut self) -> &mut Pilha {
        &mut self.saida
    }

    /// Tira da bandeja o que saiu, para a pessoa levar embora ou guardar.
    pub fn retirar_saida(&mut self) -> Pilha {
        std::mem::take(&mut self.saida)
    }

    /// Acrescenta papel em branco por cima da entrada.
    pub fn abastecer(&mut self, papel: usize) {
        let cabe = LIMITE_FOLHAS.saturating_sub(self.entrada.len());
        self.entrada
            .acrescentar_por_cima(Pilha::de_papel(papel.min(cabe)));
    }

    fn topo_da_imagem_no_mundo(&self, giro: Giro) -> Vetor {
        let topo = match self.modo.topo_da_imagem {
            TopoDaImagem::NaArestaQueEntra => Vetor::Y,
            TopoDaImagem::NaArestaQueSai => Vetor::Y.oposto(),
        };
        giro.aplicar(topo, self.modo.imprime_na_face_para.normal())
    }

    fn gravar(&self, folha: &mut Folha, envio: Envio) {
        if let Some(pagina) = envio.pagina {
            let topo = self.topo_da_imagem_no_mundo(envio.giro);
            folha.gravar(self.modo.imprime_na_face_para, pagina, topo);
        }
    }

    /// Uma passada de uma face: cada envio puxa uma folha, grava, percorre o caminho e cai.
    pub fn imprimir(&mut self, envios: &[Envio]) -> ResumoDaPassada {
        let mut resumo = ResumoDaPassada::default();
        for envio in envios {
            let Some(mut folha) = self.entrada.puxar(self.modo.puxa) else {
                resumo.sem_folha.push(*envio);
                continue;
            };
            self.gravar(&mut folha, *envio);
            folha.mover(self.modo.caminho());
            self.saida.receber(folha, self.modo.cai);
        }
        resumo
    }

    /// Duas faces automáticas: frente, volta da folha por dentro da máquina, verso.
    ///
    /// A volta inverte a folha como a curva em U: a aresta de trás passa a entrar primeiro. A
    /// união é cumprida pela máquina, que escolhe o sentido da imagem do verso a partir da
    /// posição que a frente ocupa depois da volta.
    pub fn imprimir_duas_faces(
        &mut self,
        frentes_e_versos: &[(Envio, Envio)],
        uniao: Uniao,
    ) -> ResumoDaPassada {
        let mut resumo = ResumoDaPassada::default();
        for (frente, verso) in frentes_e_versos {
            let Some(mut folha) = self.entrada.puxar(self.modo.puxa) else {
                resumo.sem_folha.push(*frente);
                resumo.sem_folha.push(*verso);
                continue;
            };
            let topo_frente = self.topo_da_imagem_no_mundo(Giro::Nenhum);
            let no_material = folha.pose.desfazer(topo_frente);
            self.gravar(&mut folha, *frente);
            folha.mover(Rotacao::MEIA_VOLTA_X);
            if let Some(pagina) = verso.pagina {
                let referencia = folha.pose.aplicar(no_material);
                let alinhado = match uniao {
                    Uniao::PelaArestaLonga => referencia,
                    Uniao::PelaArestaCurta => referencia.oposto(),
                };
                let normal = self.modo.imprime_na_face_para.normal();
                let topo = verso.giro.aplicar(alinhado, normal);
                folha.gravar(self.modo.imprime_na_face_para, pagina, topo);
            }
            folha.mover(self.modo.caminho());
            self.saida.receber(folha, self.modo.cai);
        }
        resumo
    }

    /// Cumpre o gesto: tira a pilha da saída, faz as operações e coloca na entrada. Se o gesto
    /// não puder ser cumprido, a pilha fica onde está.
    pub fn recolocar(&mut self, gesto: Gesto) -> Result<Vec<Operacao>, ErroGesto> {
        if gesto.entrada != self.modo.entrada {
            return Err(ErroGesto::EntradaDiferente {
                pedida: gesto.entrada,
                da_maquina: self.modo.entrada,
            });
        }
        let operacoes = gesto.operacoes(&self.saida)?;
        self.recolocar_com(&operacoes);
        Ok(operacoes)
    }

    /// Recoloca a pilha fazendo exatamente estas operações, sem nenhum julgamento.
    pub fn recolocar_com(&mut self, operacoes: &[Operacao]) {
        let mut pilha = std::mem::take(&mut self.saida);
        pilha.aplicar_todas(operacoes);
        self.entrada.acrescentar_por_cima(pilha);
    }
}
