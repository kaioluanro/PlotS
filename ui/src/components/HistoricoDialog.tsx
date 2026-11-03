import * as Dialog from "@radix-ui/react-dialog";
import { Check, CircleAlert } from "lucide-react";
import { useEffect, useId, useRef, useState } from "react";
import { rolarPeloTeclado } from "../dialogo";
import {
  definirGuardarHistorico,
  lerGuardarHistorico,
  linhasDasImpressoras,
  listarHistorico,
  nomeDaVelocidade,
  registrosPorData,
  textoDaData,
  textoDaVelocidade,
  textoDoResultado,
  textoDoTamanho,
  type ListaDoHistorico,
} from "../historico";
import { t } from "../i18n";
import type { Impressora } from "../impressoras";
import { FalhaDaOpcao } from "./BlocoAjuste";
import botoes from "./Botao.module.css";
import dialogo from "./ConfirmDialog.module.css";
import styles from "./HistoricoDialog.module.css";

type Leitura = { estado: "lendo" } | { estado: "falhou" } | { estado: "pronta"; lista: ListaDoHistorico };

interface Props {
  aberto: boolean;
  impressoras: readonly Impressora[];
  falhaAoApagar: boolean;
  /** Muda a cada apagamento: a lista e lida de novo. */
  versao: number;
  aoFechar: () => void;
  aoPedirApagar: () => void;
}

export type EstadoDaGuarda = "lendo" | "ligada" | "desligada" | "ilegivel";

/** O controle de guardar o historico, em cada estado. */
export function VistaGuardarHistorico({
  estado,
  falhou,
  aoMudar,
}: {
  estado: EstadoDaGuarda;
  falhou: boolean;
  aoMudar: (ligada: boolean) => void;
}) {
  const idFrase = useId();
  const ligada = estado === "ligada";
  let frase: string | null = null;
  if (estado === "desligada") frase = t("historico.guardar.desligada");
  if (estado === "ilegivel") frase = t("historico.guardar.falhaLeitura");
  return (
    <div className={styles.opcoes}>
      <label className={styles.controle}>
        <span>{t("historico.guardar.rotulo")}</span>
        <span className={styles.alvoCaixa}>
          <input
            type="checkbox"
            className={styles.caixa}
            checked={ligada}
            disabled={estado === "lendo" || estado === "ilegivel"}
            aria-describedby={frase === null ? undefined : idFrase}
            onChange={(evento) => aoMudar(evento.currentTarget.checked)}
          />
          {ligada && <Check size={12} strokeWidth={3} className={styles.glifo} aria-hidden="true" />}
        </span>
      </label>
      {frase !== null && (
        <p id={idFrase} className={estado === "ilegivel" ? styles.falhaOpcao : styles.dicaOpcao}>
          {estado === "ilegivel" && (
            <CircleAlert size={12} className={styles.iconeFalha} aria-hidden="true" />
          )}
          <span>{frase}</span>
        </p>
      )}
      {falhou && <FalhaDaOpcao />}
    </div>
  );
}

/** Le a escolha ao abrir e grava cada mudanca, uma de cada vez, para a ultima ser a que fica. */
function OpcaoGuardarHistorico({ aberto }: { aberto: boolean }) {
  const [estado, setEstado] = useState<EstadoDaGuarda>("lendo");
  const [falhou, setFalhou] = useState(false);
  const gravacoes = useRef<Promise<unknown>>(Promise.resolve());

  useEffect(() => {
    if (!aberto) return undefined;
    let valida = true;
    setEstado("lendo");
    setFalhou(false);
    gravacoes.current
      .then(lerGuardarHistorico)
      .then((ligada) => {
        if (valida) setEstado(ligada ? "ligada" : "desligada");
      })
      .catch(() => {
        if (valida) setEstado("ilegivel");
      });
    return () => {
      valida = false;
    };
  }, [aberto]);

  return (
    <VistaGuardarHistorico
      estado={estado}
      falhou={falhou}
      aoMudar={(ligada) => {
        setEstado(ligada ? "ligada" : "desligada");
        setFalhou(false);
        const pedido = gravacoes.current.then(() => definirGuardarHistorico(ligada));
        gravacoes.current = pedido.catch(() => {
          // A caixa volta ao que ficou gravado, que e o contrario do que acabou de ser pedido.
          setEstado(ligada ? "desligada" : "ligada");
          setFalhou(true);
        });
      }}
    />
  );
}

/** Todo texto aqui veio do arquivo: entra como texto, e nunca decide nada. */
export function HistoricoDialog({
  aberto,
  impressoras,
  falhaAoApagar,
  versao,
  aoFechar,
  aoPedirApagar,
}: Props) {
  const [leitura, setLeitura] = useState<Leitura>({ estado: "lendo" });
  const botaoFechar = useRef<HTMLButtonElement>(null);
  const origem = useRef<HTMLElement | null>(null);
  const corpo = useRef<HTMLDivElement>(null);

  useEffect(() => {
    if (!aberto) return undefined;
    let valida = true;
    setLeitura({ estado: "lendo" });
    listarHistorico()
      .then((lista) => {
        if (valida) setLeitura({ estado: "pronta", lista });
      })
      .catch(() => {
        if (valida) setLeitura({ estado: "falhou" });
      });
    return () => {
      valida = false;
    };
  }, [aberto, versao]);

  const pronta = leitura.estado === "pronta" ? leitura.lista : null;
  const registros = pronta ? registrosPorData(pronta.registros) : [];
  // Um arquivo danificado tambem pode ser apagado, sempre a pedido e com pergunta.
  const podeApagar = leitura.estado === "falhou" || registros.length > 0 || (pronta?.ignorados ?? 0) > 0;

  return (
    <Dialog.Root
      open={aberto}
      onOpenChange={(abrir) => {
        if (!abrir) aoFechar();
      }}
    >
      <Dialog.Portal>
        <Dialog.Overlay className={dialogo.veu} />
        <Dialog.Content
          className={`${dialogo.conteudo} ${styles.conteudo}`}
          onKeyDown={(evento) => {
            if (rolarPeloTeclado(evento.key, corpo.current)) evento.preventDefault();
          }}
          onOpenAutoFocus={(evento) => {
            if (document.activeElement instanceof HTMLElement) origem.current = document.activeElement;
            evento.preventDefault();
            botaoFechar.current?.focus();
          }}
          onCloseAutoFocus={(evento) => {
            evento.preventDefault();
            if (origem.current?.isConnected) origem.current.focus();
          }}
        >
          <Dialog.Title className={dialogo.titulo}>{t("historico.titulo")}</Dialog.Title>
          <Dialog.Description asChild>
            <div ref={corpo} className={dialogo.corpo}>
              <p className={dialogo.nota}>{t("historico.local")}</p>
              <OpcaoGuardarHistorico aberto={aberto} />
              {leitura.estado === "lendo" && (
                <p role="status" className={dialogo.texto}>
                  {t("historico.lendo")}
                </p>
              )}
              {leitura.estado === "falhou" && (
                <p role="alert" className={dialogo.texto}>
                  {t("historico.falha.leitura")}
                </p>
              )}
              {falhaAoApagar && (
                <p role="alert" className={dialogo.texto}>
                  {t("historico.falha.apagar")}
                </p>
              )}
              {pronta !== null && registros.length === 0 && pronta.ignorados === 0 && (
                <p className={dialogo.texto}>{t("historico.vazio")}</p>
              )}
              {pronta !== null && pronta.velocidades.length > 0 && (
                <section className={styles.velocidades} aria-labelledby="titulo-velocidades">
                  <h3 id="titulo-velocidades" className={styles.subtitulo}>
                    {t("historico.velocidades")}
                  </h3>
                  <ul className={styles.lista}>
                    {pronta.velocidades.map((medida) => {
                      const nome = nomeDaVelocidade(medida, pronta.velocidades, impressoras);
                      return (
                      <li key={`${medida.impressora}-${String(medida.automatico)}`} className={styles.parValor}>
                        <span className={styles.nomeImpressora} title={nome}>
                          {nome}
                        </span>
                        <span className={styles.valor}>{textoDaVelocidade(medida.folhas_por_minuto)}</span>
                      </li>
                      );
                    })}
                  </ul>
                </section>
              )}
              {registros.length > 0 && (
                <ol className={styles.lista}>
                  {registros.map((registro, indice) => (
                    <li key={`${registro.data}-${indice}`} className={styles.registro}>
                      <div className={styles.cabecalho}>
                        <span className={styles.arquivo} title={registro.arquivo}>
                          {registro.arquivo}
                        </span>
                        <span className={styles.data}>{textoDaData(registro.data)}</span>
                      </div>
                      <p className={styles.resultado}>{textoDoResultado(registro)}</p>
                      <p className={styles.detalhe}>{textoDoTamanho(registro)}</p>
                      {linhasDasImpressoras(registro, impressoras).map((linha, ordem) => (
                        <p key={ordem} className={styles.detalhe}>
                          {linha}
                        </p>
                      ))}
                    </li>
                  ))}
                </ol>
              )}
              {pronta !== null && pronta.ignorados > 0 && (
                <p className={dialogo.nota}>
                  {t("historico.ignorados", { n: String(pronta.ignorados) })}
                </p>
              )}
            </div>
          </Dialog.Description>
          <div className={dialogo.rodape}>
            {podeApagar && (
              <button
                type="button"
                className={`${botoes.botao} ${botoes.secundario}`}
                onClick={aoPedirApagar}
              >
                {t("historico.apagar")}
              </button>
            )}
            <Dialog.Close ref={botaoFechar} className={`${botoes.botao} ${botoes.primario}`}>
              {t("historico.fechar")}
            </Dialog.Close>
          </div>
        </Dialog.Content>
      </Dialog.Portal>
    </Dialog.Root>
  );
}
