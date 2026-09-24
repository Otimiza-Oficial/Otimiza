"use client";

import { AnimatePresence, motion } from "framer-motion";
import { Check, RotateCcw } from "lucide-react";
import { useEffect, useState, useSyncExternalStore } from "react";
import { useReducedMotionSafe } from "@/components/motion/useReducedMotionSafe";
import { Aviso } from "@/components/ui/Aviso";
import { API, consultar, misturaInsegura, TEM_CARTAO, type Compra, type Consulta, type Metodo } from "@/lib/api";
import { codigoValido, normalizarCodigo } from "@/lib/licenca";
import { PRECO_BRL } from "@/lib/site";
import { PassoCartao, type DadosDoCartao } from "./PassoCartao";
import { PassoChave } from "./PassoChave";
import { PassoCodigo } from "./PassoCodigo";
import { PassoMetodo } from "./PassoMetodo";
import { PassoPix } from "./PassoPix";
import { sessao, useAcompanhar } from "./useAcompanhar";

type Passo =
  | { nome: "codigo" }
  | { nome: "metodo" }
  | { nome: "pix"; compra: Compra }
  | { nome: "cartao" }
  | { nome: "analise"; compra: Compra }
  | { nome: "chave"; chave: string }
  | { nome: "fim"; mensagem: string };

const ETAPAS = ["Código", "Pagamento", "Chave"] as const;

function etapaDe(passo: Passo["nome"]) {
  if (passo === "codigo") return 0;
  if (passo === "chave") return 2;
  return 1;
}

const MENSAGENS: Partial<Record<Consulta["estado"], string>> = {
  vencida: "O prazo deste pagamento acabou antes de ele ser confirmado. Nada foi cobrado; gere outro quando quiser.",
  cancelada: "Esta cobrança foi cancelada. Nada foi cobrado.",
  estornada: "Este pagamento foi estornado. Se não foi você, fale no Discord.",
  recusada: "O pagamento foi recusado. Nada foi cobrado.",
  entregue: "A chave desta compra já foi entregue. Se você a perdeu, o suporte reemite no Discord, sem custo.",
};

export function Checkout() {
  const reduce = useReducedMotionSafe();
  const { acompanhar, restante, parar } = useAcompanhar();
  const [passo, setPasso] = useState<Passo>({ nome: "codigo" });
  const doEndereco = useSyncExternalStore(semAssinatura, codigoDoEndereco, () => "");
  const [codigoDigitado, setCodigo] = useState("");
  const codigo = codigoDigitado || doEndereco;
  const [carregando, setCarregando] = useState<Metodo | null>(null);
  const [erro, setErro] = useState<string | null>(null);

  function aoTerminar(consulta: Consulta) {
    sessao.limpar();
    if (consulta.estado === "pago" && consulta.chave) setPasso({ nome: "chave", chave: consulta.chave });
    else setPasso({ nome: "fim", mensagem: MENSAGENS[consulta.estado] ?? "Algo deu errado com este pagamento." });
  }

  function esperar(compra: Compra, metodo: Metodo, maquina: string) {
    sessao.gravar({ compra, metodo, maquina });
    setPasso(metodo === "pix" ? { nome: "pix", compra } : { nome: "analise", compra });
    acompanhar(compra, aoTerminar);
  }

  useEffect(() => {
    const guardada = sessao.ler();
    if (!guardada) return;
    consultar(guardada.compra.token)
      .then((consulta) => {
        if (!consulta) return sessao.limpar();
        setCodigo(guardada.maquina);
        if (consulta.estado === "pendente" || consulta.estado === "em_analise") {
          esperar(guardada.compra, guardada.metodo, guardada.maquina);
        } else {
          aoTerminar(consulta);
        }
      })
      .catch(() => {});
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, []);

  async function escolher(metodo: Metodo) {
    setErro(null);
    if (metodo === "cartao") return setPasso({ nome: "cartao" });

    setCarregando("pix");
    try {
      const r = await fetch(`${API}/v1/compras`, {
        method: "POST",
        headers: { "content-type": "application/json", accept: "application/json" },
        body: JSON.stringify({ maquina: codigo }),
      });
      const corpo = await r.json().catch(() => null);
      if (r.status === 201 && corpo?.token) esperar(corpo as Compra, "pix", codigo);
      else setErro(corpo?.descricao ?? falhaGenerica(r.status));
    } catch {
      setErro("Não consegui falar com o pagamento. Confira a sua conexão, ou compre pelo Discord.");
    } finally {
      setCarregando(null);
    }
  }

  async function pagarComCartao(dados: DadosDoCartao) {
    setErro(null);
    try {
      const r = await fetch(`${API}/v1/compras/cartao`, {
        method: "POST",
        headers: { "content-type": "application/json", accept: "application/json" },
        body: JSON.stringify({ maquina: codigo, ...dados }),
      });
      const corpo = await r.json().catch(() => null);

      if (corpo?.estado === "pago" && corpo.chave) {
        sessao.limpar();
        setPasso({ nome: "chave", chave: corpo.chave });
      } else if (corpo?.estado === "em_analise" && corpo.token) {
        esperar(corpo as Compra, "cartao", codigo);
      } else if (corpo?.estado === "recusada") {
        setErro(corpo.motivo ?? "O banco recusou este cartão. Nada foi cobrado. Confira os dados ou use outro cartão.");
      } else {
        setErro(corpo?.descricao ?? falhaGenerica(r.status));
      }
    } catch {
      setErro("Não consegui falar com o pagamento. Nada foi cobrado. Tente de novo em instantes.");
    }
  }

  function recomecar() {
    parar();
    sessao.limpar();
    setErro(null);
    setPasso({ nome: "codigo" });
  }

  if (misturaInsegura()) {
    return <Aviso>O endereço do pagamento está sem HTTPS e o navegador bloqueia a chamada. Por enquanto, compre pelo Discord.</Aviso>;
  }

  const etapa = etapaDe(passo.nome);

  return (
    <div>
      <ol className="mb-6 flex items-center gap-2" aria-label="Etapas da compra">
        {ETAPAS.map((nome, i) => (
          <li key={nome} className="flex flex-1 items-center gap-2" aria-current={i === etapa ? "step" : undefined}>
            <span
              className={`grid size-6 shrink-0 place-items-center rounded-full font-mono text-[11px] transition-colors ${
                i <= etapa ? "bg-ink text-white" : "bg-card text-subtle"
              }`}
            >
              {i < etapa ? <Check size={12} strokeWidth={3} aria-hidden="true" /> : i + 1}
            </span>
            <span className={`text-[12.5px] font-medium ${i <= etapa ? "text-fg" : "text-subtle"}`}>{nome}</span>
            {i < ETAPAS.length - 1 && (
              <span className="h-px flex-1 bg-line" aria-hidden="true">
                <motion.span
                  className="block h-px origin-left bg-fg"
                  initial={false}
                  animate={{ scaleX: i < etapa ? 1 : 0 }}
                  transition={{ duration: reduce ? 0 : 0.4, ease: [0.22, 1, 0.36, 1] }}
                />
              </span>
            )}
          </li>
        ))}
      </ol>

      <AnimatePresence mode="wait" initial={false}>
        <motion.div
          key={passo.nome}
          initial={{ opacity: 0, x: reduce ? 0 : 12 }}
          animate={{ opacity: 1, x: 0 }}
          exit={{ opacity: 0, x: reduce ? 0 : -12 }}
          transition={{ duration: reduce ? 0 : 0.22, ease: [0.22, 1, 0.36, 1] }}
        >
          {passo.nome === "codigo" && (
            <PassoCodigo
              key={codigo}
              inicial={codigo}
              aoConfirmar={(c) => {
                setCodigo(c);
                setPasso({ nome: "metodo" });
              }}
            />
          )}
          {passo.nome === "metodo" && (
            <PassoMetodo
              codigo={codigo}
              temCartao={TEM_CARTAO}
              carregando={carregando}
              erro={erro}
              aoEscolher={escolher}
              aoTrocarCodigo={() => setPasso({ nome: "codigo" })}
            />
          )}
          {passo.nome === "pix" && <PassoPix compra={passo.compra} restante={restante} aoVoltar={recomecarNoMetodo} />}
          {passo.nome === "cartao" && (
            <PassoCartao valor={PRECO_BRL} erro={erro} aoPagar={pagarComCartao} aoVoltar={recomecarNoMetodo} />
          )}
          {passo.nome === "analise" && (
            <Aviso>
              O banco está analisando este pagamento. Pode deixar esta aba aberta: a chave aparece aqui assim que ele
              for aprovado.
            </Aviso>
          )}
          {passo.nome === "chave" && <PassoChave chave={passo.chave} />}
          {passo.nome === "fim" && (
            <div className="space-y-4">
              <Aviso>{passo.mensagem}</Aviso>
              <button
                type="button"
                onClick={recomecar}
                className="inline-flex h-11 items-center gap-2 rounded-[var(--radius-control)] bg-ink px-4 text-[13.5px] font-semibold text-white"
              >
                <RotateCcw size={15} aria-hidden="true" />
                Começar de novo
              </button>
            </div>
          )}
        </motion.div>
      </AnimatePresence>
    </div>
  );

  function recomecarNoMetodo() {
    parar();
    sessao.limpar();
    setErro(null);
    setPasso({ nome: "metodo" });
  }
}

const semAssinatura = () => () => {};

function codigoDoEndereco() {
  const codigo = normalizarCodigo(new URLSearchParams(window.location.search).get("maquina") ?? "");
  return codigoValido(codigo) ? codigo : "";
}

function falhaGenerica(status: number) {
  if (status === 429) return "Muitas tentativas seguidas. Espere um minuto e tente de novo.";
  if (status >= 500) return "O pagamento está fora do ar neste momento. Nada foi cobrado. A compra pelo Discord continua funcionando.";
  return "Não consegui criar a cobrança. Tente de novo, ou compre pelo Discord.";
}
