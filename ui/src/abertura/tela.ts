import { invoke } from "@tauri-apps/api/core";
import splash from "../../assets/splash.png";
import { idiomaAtual, t } from "../i18n";
import { acompanharTemaDoSistema } from "../theme";
import "../theme.css";
import "./tela.css";
import { fracaoDoProgresso, linhaDeEstado, textoDoTempo, type Andamento } from "./textos";

const INTERVALO_MS = 100;

document.documentElement.lang = idiomaAtual;
acompanharTemaDoSistema();

function montar(raiz: HTMLElement) {
  raiz.className = "abertura";

  // A imagem traz o nome, a frase e o autor; o texto alternativo diz o mesmo para quem nao a ve.
  const imagem = document.createElement("img");
  imagem.className = "abertura-imagem";
  imagem.src = splash;
  imagem.alt = t("abertura.imagem.alt");
  imagem.width = 480;
  imagem.height = 300;
  imagem.draggable = false;

  const barra = document.createElement("div");
  barra.className = "abertura-barra";
  barra.setAttribute("role", "progressbar");
  barra.setAttribute("aria-labelledby", "abertura-estado-texto");
  barra.setAttribute("aria-valuemin", "0");
  barra.setAttribute("aria-valuemax", "100");
  const preenchimento = document.createElement("span");
  barra.append(preenchimento);

  const linha = document.createElement("div");
  linha.className = "abertura-estado";
  const texto = document.createElement("span");
  texto.id = "abertura-estado-texto";
  texto.setAttribute("role", "status");
  const tempo = document.createElement("span");
  tempo.className = "abertura-tempo";
  tempo.setAttribute("aria-hidden", "true");
  linha.append(texto, tempo);

  // O andamento fica sobre a faixa vazia no pe da imagem.
  const rodape = document.createElement("div");
  rodape.className = "abertura-rodape";
  rodape.append(barra, linha);

  raiz.append(imagem, rodape);

  let etapaAtual: string | null = null;
  let desde = Date.now();
  let fracaoAtual = -1;

  // Escreve so o que mudou, para o leitor de tela nao repetir a mesma linha.
  return (andamento: Pick<Andamento, "etapa" | "concluidas" | "total">) => {
    if (andamento.etapa !== etapaAtual) {
      etapaAtual = andamento.etapa;
      desde = Date.now();
      texto.textContent = linhaDeEstado(andamento.etapa);
    }
    const fracao = fracaoDoProgresso(andamento);
    if (fracao !== fracaoAtual) {
      fracaoAtual = fracao;
      preenchimento.style.transform = `scaleX(${fracao})`;
      barra.setAttribute("aria-valuenow", String(Math.round(fracao * 100)));
    }
    const decorrido = textoDoTempo(Date.now() - desde) ?? "";
    if (tempo.textContent !== decorrido) tempo.textContent = decorrido;
  };
}

const raiz = document.getElementById("abertura");
if (raiz) {
  const mostrar = montar(raiz);
  let ultimo: Andamento = { etapa: "impressoras", concluidas: 0, total: 1 };
  mostrar(ultimo);
  const consultar = () => {
    invoke<Andamento>("andamento_da_abertura")
      .then((novo) => {
        ultimo = novo;
        mostrar(novo);
      })
      .catch(() => {
        // Sem resposta, a linha anterior continua; o contador de segundos segue andando.
        mostrar(ultimo);
      });
  };
  window.setInterval(consultar, INTERVALO_MS);
  consultar();
}
