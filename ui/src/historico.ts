import { invoke } from "@tauri-apps/api/core";
import type { ConteudoConfirmacao } from "./confirmacao";
import { idiomaAtual, traduzir, type Chave, type Idioma } from "./i18n";
import { nomeDeExibicao, type Impressora } from "./impressoras";

export type ResultadoDoRegistro = "concluido" | "cancelado" | "com_erro" | "incompleto";

export interface TempoDoRegistro {
  impressora: string;
  milissegundos: number;
  folhas: number;
  resultado: ResultadoDoRegistro;
  automatico?: boolean;
}

export interface RegistroDoHistorico {
  arquivo: string;
  data: number;
  impressoras: string[];
  intervalo?: [number, number];
  paginas: number;
  lotes: number;
  resultado: ResultadoDoRegistro;
  lotes_faltando: number;
  tempos?: TempoDoRegistro[];
}

export interface VelocidadeDoRegistro {
  impressora: string;
  folhas_por_minuto: number;
  automatico?: boolean;
}

export interface ListaDoHistorico {
  registros: RegistroDoHistorico[];
  ignorados: number;
  velocidades: VelocidadeDoRegistro[];
}

export function listarHistorico(): Promise<ListaDoHistorico> {
  return invoke<ListaDoHistorico>("listar_historico");
}

export function apagarHistorico(): Promise<void> {
  return invoke<void>("apagar_historico");
}

/** Se os proximos envios entram na lista. A recusa quer dizer que a escolha nao pode ser lida. */
export function lerGuardarHistorico(): Promise<boolean> {
  return invoke<boolean>("ler_guardar_historico");
}

export function definirGuardarHistorico(ligado: boolean): Promise<void> {
  return invoke<void>("definir_guardar_historico", { ligado });
}

/** Os mais recentes primeiro. Na mesma data, vale a ordem em que terminaram. */
export function registrosPorData(registros: readonly RegistroDoHistorico[]): RegistroDoHistorico[] {
  return registros
    .map((registro, indice) => ({ registro, indice }))
    .sort((a, b) => b.registro.data - a.registro.data || b.indice - a.indice)
    .map(({ registro }) => registro);
}

const CHAVES_DO_RESULTADO: Record<ResultadoDoRegistro, Chave> = {
  concluido: "historico.resultado.concluido",
  cancelado: "historico.resultado.cancelado",
  com_erro: "historico.resultado.comErro",
  incompleto: "historico.resultado.incompleto",
};

export function textoDoResultado(
  registro: RegistroDoHistorico,
  idioma: Idioma = idiomaAtual,
): string {
  const resultado = traduzir(idioma, CHAVES_DO_RESULTADO[registro.resultado]);
  if (registro.lotes_faltando <= 0) return resultado;
  const faltando = traduzir(
    idioma,
    registro.lotes_faltando === 1 ? "historico.faltando.um" : "historico.faltando.varios",
    { n: String(registro.lotes_faltando) },
  );
  return `${resultado} · ${faltando}`;
}

/** Paginas, trecho e lotes em uma linha. */
export function textoDoTamanho(
  registro: RegistroDoHistorico,
  idioma: Idioma = idiomaAtual,
): string {
  const partes = [
    traduzir(idioma, registro.paginas === 1 ? "historico.paginas.um" : "historico.paginas.varios", {
      n: String(registro.paginas),
    }),
  ];
  if (registro.intervalo) {
    partes.push(
      traduzir(idioma, "historico.intervalo", {
        inicio: String(registro.intervalo[0]),
        fim: String(registro.intervalo[1]),
      }),
    );
  }
  partes.push(
    traduzir(idioma, registro.lotes === 1 ? "historico.lotes.um" : "historico.lotes.varios", {
      n: String(registro.lotes),
    }),
  );
  return partes.join(" · ");
}

/** Sem data que se leia, o registro continua na lista, com um traco no lugar dela. */
export function textoDaData(segundos: number, idioma: Idioma = idiomaAtual): string {
  const data = new Date(segundos * 1000);
  if (Number.isNaN(data.getTime())) return "—";
  return new Intl.DateTimeFormat(idioma, { dateStyle: "short", timeStyle: "short" }).format(data);
}

export function textoDoTempo(milissegundos: number, idioma: Idioma = idiomaAtual): string {
  const total = Math.max(1, Math.round(milissegundos / 1000));
  const minutos = Math.floor(total / 60);
  const segundos = total % 60;
  if (minutos === 0) return traduzir(idioma, "historico.tempo.segundos", { s: String(segundos) });
  return traduzir(idioma, "historico.tempo.minutos", { m: String(minutos), s: String(segundos) });
}

export function textoDaVelocidade(folhasPorMinuto: number, idioma: Idioma = idiomaAtual): string {
  const valor = new Intl.NumberFormat(idioma, { maximumFractionDigits: 1 }).format(folhasPorMinuto);
  return traduzir(idioma, "historico.velocidade", { valor });
}

/** O nome como a lista mostra; a impressora que sumiu fica com o nome guardado, sem o prefixo. */
export function nomeNoHistorico(
  id: string,
  impressoras: readonly Impressora[],
  idioma: Idioma = idiomaAtual,
): string {
  const atual = impressoras.find((impressora) => impressora.id === id);
  if (atual) return nomeDeExibicao(atual, idioma);
  return id.startsWith("fila:") ? id.slice("fila:".length) : id;
}

/** Uma linha por impressora: com o tempo, se foi medido, e so o nome, se nao foi. */
export function linhasDasImpressoras(
  registro: RegistroDoHistorico,
  impressoras: readonly Impressora[],
  idioma: Idioma = idiomaAtual,
): string[] {
  const tempos = registro.tempos ?? [];
  const ids = new Set([...registro.impressoras, ...tempos.map((tempo) => tempo.impressora)]);
  return [...ids].map((id) => {
    const nome = nomeNoHistorico(id, impressoras, idioma);
    // Um envio misto tem uma medida por rota; a linha diz o tempo que a impressora trabalhou.
    const medidas = tempos.filter((tempo) => tempo.impressora === id);
    if (medidas.length === 0) return nome;
    return traduzir(idioma, "historico.tempo", {
      nome,
      tempo: textoDoTempo(
        medidas.reduce((soma, tempo) => soma + tempo.milissegundos, 0),
        idioma,
      ),
    });
  });
}

/** Com medida nas duas rotas, o nome diz de qual se trata. A automatica sempre diz. */
export function nomeDaVelocidade(
  medida: VelocidadeDoRegistro,
  todas: readonly VelocidadeDoRegistro[],
  impressoras: readonly Impressora[],
  idioma: Idioma = idiomaAtual,
): string {
  const nome = nomeNoHistorico(medida.impressora, impressoras, idioma);
  if (medida.automatico === true) return traduzir(idioma, "historico.velocidade.automatica", { nome });
  const temAutomatica = todas.some(
    (outra) => outra.impressora === medida.impressora && outra.automatico === true,
  );
  return temAutomatica ? traduzir(idioma, "historico.velocidade.manual", { nome }) : nome;
}

/** Apagar tira tudo de uma vez: sempre pergunta. */
export function conteudoDoApagarHistorico(): ConteudoConfirmacao {
  return {
    titulo: "historico.apagarDialogo.titulo",
    texto: "historico.apagarDialogo.texto",
    consequencia: null,
    manter: "cartao.envio.voltar",
    fechar: "historico.apagarDialogo.confirmar",
    parametros: {},
    ehJanela: false,
    focoNaOrigemAoConfirmar: true,
  };
}
