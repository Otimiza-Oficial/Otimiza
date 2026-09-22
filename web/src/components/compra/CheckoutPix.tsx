"use client";

import { Check, Copy, KeyRound, Loader2, QrCode, TriangleAlert } from "lucide-react";
import Image from "next/image";
import Link from "next/link";
import { useCallback, useEffect, useRef, useState } from "react";
import { API, INTERVALO_CONSULTA_MS, LIMITE_CONSULTA_MS, misturaInsegura, type Compra, type Consulta } from "@/lib/api";
import { codigoValido, normalizarCodigo } from "@/lib/licenca";
import { useLicencas } from "@/lib/armazem";

/*
 * O CHECKOUT PIX, NO SITE QUE ESTÁ NO AR.
 *
 * Ele nasceu no site antigo (Astro) e nunca chegou a ligar. Aqui ele é o mesmo
 * contrato — `POST /v1/compras` e `GET /v1/compras/{token}` no serviço do bot —
 * com as mesmas decisões, que valem a pena repetir por escrito:
 *
 * O QUE FICA GUARDADO: só o token da compra em andamento, em `sessionStorage`,
 * e só até a chave sair. É o que permite recarregar a página sem perder um Pix
 * já pago — o caso em que a pessoa mais precisaria do suporte se o dado
 * sumisse. `sessionStorage` e não `localStorage` de propósito: o token dá
 * acesso a uma licença e não deve sobreviver ao fechamento da aba num
 * computador que pode ser de outra pessoa.
 *
 * A CHAVE APARECE UMA VEZ, e a tela diz isso antes de ela aparecer. Junto vem
 * o botão que a guarda no navegador (a mesma gaveta do painel) — sem isso,
 * fechar a aba custa uma ida ao suporte.
 *
 * NADA DE CARTÃO AQUI. Este componente nunca vê dado de pagamento: quem cobra
 * é o provedor, pelo Pix, fora do site.
 */

type Fase =
  | { nome: "formulario" }
  | { nome: "pix"; compra: Compra }
  | { nome: "pago"; chave: string }
  | { nome: "encerrada"; motivo: string };

const CHAVE_SESSAO = "otimiza.compra";

/** O token sai da gaveta assim que deixa de servir. Ele dá acesso a uma licença. */
function limparSessao() {
  try {
    sessionStorage.removeItem(CHAVE_SESSAO);
  } catch {}
}

export function CheckoutPix({ codigoInicial = "" }: { codigoInicial?: string }) {
  const [codigo, setCodigo] = useState(codigoInicial);
  const [fase, setFase] = useState<Fase>({ nome: "formulario" });
  const [erro, setErro] = useState<string | null>(null);
  const [enviando, setEnviando] = useState(false);
  const [restante, setRestante] = useState<number | null>(null);
  const parar = useRef<(() => void) | null>(null);

  const insegura = misturaInsegura();

  /** Pergunta ao serviço, de três em três segundos, se o Pix foi pago. */
  const acompanhar = useCallback((token: string, expiraEm: string) => {
    parar.current?.();
    const comecou = Date.now();
    const fim = new Date(expiraEm).getTime();

    const relogio = setInterval(() => {
      const falta = Math.max(0, Math.round((fim - Date.now()) / 1000));
      setRestante(Number.isFinite(falta) ? falta : null);
    }, 1000);

    const consulta = setInterval(async () => {
      if (Date.now() - comecou > LIMITE_CONSULTA_MS) {
        encerrar("O tempo do Pix acabou. Gere outro, ou fale no Discord — o suporte resolve na mão.");
        return;
      }
      try {
        const r = await fetch(`${API}/v1/compras/${encodeURIComponent(token)}`, { headers: { accept: "application/json" } });
        if (!r.ok) return;
        const corpo: Consulta = await r.json();
        if (corpo.estado === "pago" && corpo.chave) {
          limparSessao();
          setFase({ nome: "pago", chave: corpo.chave });
          pararTudo();
        } else if (corpo.estado === "estornada") {
          encerrar("Este pagamento foi estornado. Se não foi você, fale no Discord.");
        } else if (corpo.estado === "vencida" || corpo.estado === "cancelada") {
          encerrar("Este Pix venceu antes de ser pago. Gere outro quando quiser.");
        }
      } catch {
        // Rede oscilou: a próxima volta tenta de novo. Um erro aqui não pode
        // apagar da tela um Pix que talvez já tenha sido pago.
      }
    }, INTERVALO_CONSULTA_MS);

    const pararTudo = () => {
      clearInterval(relogio);
      clearInterval(consulta);
      parar.current = null;
    };
    const encerrar = (motivo: string) => {
      limparSessao();
      pararTudo();
      setRestante(null);
      setFase({ nome: "encerrada", motivo });
    };
    parar.current = pararTudo;
  }, []);

  /* Recarregou a página com um Pix em andamento? Retoma de onde parou. */
  useEffect(() => {
    let guardado: { token?: string; expiraEm?: string } | null = null;
    try {
      guardado = JSON.parse(sessionStorage.getItem(CHAVE_SESSAO) ?? "null");
    } catch {}
    if (!guardado?.token) return;

    (async () => {
      try {
        const r = await fetch(`${API}/v1/compras/${encodeURIComponent(guardado.token!)}`, { headers: { accept: "application/json" } });
        if (!r.ok) return limparSessao();
        const corpo: Consulta = await r.json();
        if (corpo.estado === "pago" && corpo.chave) {
          limparSessao();
          setFase({ nome: "pago", chave: corpo.chave });
        } else if (corpo.estado === "pendente" && guardado.expiraEm) {
          setFase({ nome: "pix", compra: { token: guardado.token!, expiraEm: guardado.expiraEm } });
          acompanhar(guardado.token!, guardado.expiraEm);
        } else {
          limparSessao();
        }
      } catch {
        // Sem resposta agora: o formulário continua valendo.
      }
    })();

    return () => parar.current?.();
  }, [acompanhar]);

  async function gerar(evento: React.FormEvent) {
    evento.preventDefault();
    setErro(null);
    const limpo = normalizarCodigo(codigo);
    if (!codigoValido(limpo)) {
      setErro("O código tem o formato OTZ-XXXX-XXXX-XXXX e aparece na tela de ativação do Otimiza.");
      return;
    }
    setCodigo(limpo);
    setEnviando(true);
    try {
      const r = await fetch(`${API}/v1/compras`, {
        method: "POST",
        headers: { "content-type": "application/json", accept: "application/json" },
        body: JSON.stringify({ maquina: limpo }),
      });
      const corpo = await r.json().catch(() => null);

      if (r.status === 201 && corpo?.token) {
        try {
          sessionStorage.setItem(CHAVE_SESSAO, JSON.stringify({ token: corpo.token, expiraEm: corpo.expiraEm }));
        } catch {}
        setFase({ nome: "pix", compra: corpo as Compra });
        acompanhar(corpo.token, corpo.expiraEm);
      } else if (r.status === 400 && corpo?.descricao) {
        setErro(String(corpo.descricao));
      } else if (r.status === 502 || r.status === 503) {
        setErro("O pagamento está fora do ar neste momento. A compra pelo Discord continua funcionando.");
      } else {
        setErro("Não consegui gerar o Pix. Tente de novo, ou compre pelo Discord.");
      }
    } catch {
      setErro("Não consegui falar com o servidor do pagamento. Confira a sua conexão, ou compre pelo Discord.");
    } finally {
      setEnviando(false);
    }
  }

  if (insegura) {
    return (
      <Aviso>
        O endereço do pagamento está configurado sem HTTPS, e o navegador bloqueia essa chamada a partir de uma página
        segura. Enquanto isso não for corrigido, a compra acontece pelo Discord.
      </Aviso>
    );
  }

  if (fase.nome === "pago") return <ChaveEmitida chave={fase.chave} />;

  if (fase.nome === "encerrada") {
    return (
      <div className="space-y-3">
        <Aviso>{fase.motivo}</Aviso>
        <button
          type="button"
          onClick={() => setFase({ nome: "formulario" })}
          className="inline-flex h-10 items-center rounded-[8px] bg-ink px-4 text-[13px] font-semibold text-white"
        >
          Gerar outro Pix
        </button>
      </div>
    );
  }

  if (fase.nome === "pix") {
    const { compra } = fase;
    return (
      <div className="grid gap-5 sm:grid-cols-[auto_minmax(0,1fr)]">
        {compra.pix?.qrBase64 ? (
          <div className="w-fit rounded-[12px] bg-white p-3 shadow-[0_0_0_1px_rgb(10_10_10/0.08)]">
            {/* `unoptimized`: é uma imagem que nasce no navegador, não um
                arquivo do site — não há o que otimizar na compilação. */}
            <Image
              src={`data:image/png;base64,${compra.pix.qrBase64}`}
              alt="QR Code do Pix"
              width={180}
              height={180}
              unoptimized
            />
          </div>
        ) : null}

        <div className="min-w-0 space-y-3">
          <p className="text-[13.5px] text-muted">
            Pague com o aplicativo do seu banco. Assim que o pagamento cair, a chave aparece nesta tela sozinha
            {restante !== null && restante > 0 ? ` — o Pix vale mais ${formatarRelogio(restante)}` : ""}.
          </p>

          <CampoCopiavel rotulo="Pix copia e cola" valor={compra.pix?.copiaECola ?? ""} />

          <p className="flex items-center gap-2 text-[12.5px] text-subtle">
            <Loader2 size={13} className="animate-spin" aria-hidden="true" />
            Esperando o pagamento. Pode deixar esta aba aberta.
          </p>
        </div>
      </div>
    );
  }

  return (
    <form onSubmit={gerar} className="space-y-3">
      <label htmlFor="compra-codigo" className="block text-[13px] font-medium">
        Código deste computador
      </label>
      <input
        id="compra-codigo"
        value={codigo}
        onChange={(e) => setCodigo(e.target.value)}
        placeholder="OTZ-XXXX-XXXX-XXXX"
        autoComplete="off"
        spellCheck={false}
        aria-describedby="compra-ajuda"
        className="h-11 w-full rounded-[8px] bg-white px-3 font-mono text-[13.5px] tracking-[0.04em] shadow-[0_0_0_1px_rgb(10_10_10/0.1)] outline-none focus:shadow-[0_0_0_2px_var(--color-fg)]"
      />
      <p id="compra-ajuda" className="text-[12.5px] text-subtle">
        A licença é emitida para UM computador. O código aparece na tela de ativação do Otimiza — instale primeiro, que
        o download é grátis.
      </p>

      {erro && <Aviso>{erro}</Aviso>}

      <button
        type="submit"
        disabled={enviando}
        className="inline-flex h-11 items-center gap-2 rounded-[8px] bg-ink px-4 text-[13.5px] font-semibold text-white disabled:opacity-60"
      >
        {enviando ? <Loader2 size={15} className="animate-spin" aria-hidden="true" /> : <QrCode size={15} aria-hidden="true" />}
        {enviando ? "Gerando o Pix…" : "Gerar o Pix"}
      </button>
    </form>
  );
}

/** A chave emitida: aparece uma vez, e a tela oferece guardá-la. */
function ChaveEmitida({ chave }: { chave: string }) {
  const { adicionar } = useLicencas();
  const [guardada, setGuardada] = useState(false);

  return (
    <div className="space-y-3">
      <p className="flex items-center gap-2 text-[14px] font-semibold">
        <Check size={16} strokeWidth={2.5} aria-hidden="true" />
        Pagamento confirmado. Esta é a sua chave.
      </p>
      <CampoCopiavel rotulo="Chave da licença" valor={chave} />
      <p className="text-[12.5px] text-subtle">
        Ela aparece aqui uma vez só. Guarde no navegador ou copie para um lugar seu — se perder, o suporte reemite no
        Discord, sem custo.
      </p>
      <div className="flex flex-wrap gap-2">
        <button
          type="button"
          onClick={() => {
            adicionar(chave);
            setGuardada(true);
          }}
          disabled={guardada}
          className="inline-flex h-10 items-center gap-2 rounded-[8px] bg-ink px-4 text-[13px] font-semibold text-white disabled:opacity-60"
        >
          <KeyRound size={14} aria-hidden="true" />
          {guardada ? "Guardada neste navegador" : "Guardar no navegador"}
        </button>
        <Link
          href="/painel/licencas/"
          className="inline-flex h-10 items-center rounded-[8px] bg-white px-4 text-[13px] font-semibold shadow-[0_0_0_1px_rgb(10_10_10/0.1)]"
        >
          Abrir o painel
        </Link>
      </div>
    </div>
  );
}

function CampoCopiavel({ rotulo, valor }: { rotulo: string; valor: string }) {
  const [copiado, setCopiado] = useState(false);

  return (
    <div>
      <p className="mb-1 text-[11.5px] font-medium text-muted">{rotulo}</p>
      <div className="flex items-stretch gap-2">
        <code className="min-w-0 flex-1 overflow-x-auto rounded-[8px] bg-white px-3 py-2.5 font-mono text-[12px] break-all shadow-[0_0_0_1px_rgb(10_10_10/0.1)]">
          {valor}
        </code>
        <button
          type="button"
          onClick={async () => {
            try {
              await navigator.clipboard.writeText(valor);
              setCopiado(true);
              setTimeout(() => setCopiado(false), 1600);
            } catch {
              setCopiado(false);
            }
          }}
          className="inline-flex h-auto shrink-0 items-center gap-1.5 rounded-[8px] bg-white px-3 text-[12.5px] font-semibold shadow-[0_0_0_1px_rgb(10_10_10/0.1)]"
        >
          {copiado ? <Check size={14} aria-hidden="true" /> : <Copy size={14} aria-hidden="true" />}
          {copiado ? "Copiado" : "Copiar"}
        </button>
      </div>
    </div>
  );
}

function Aviso({ children }: { children: React.ReactNode }) {
  return (
    <p
      role="status"
      className="flex items-start gap-2 rounded-[8px] bg-[#fffaf2] px-3 py-2.5 text-[12.5px] leading-[1.5] text-[#5b4300] shadow-[0_0_0_1px_rgb(180_120_20/0.22)]"
    >
      <TriangleAlert size={14} className="mt-0.5 shrink-0" aria-hidden="true" />
      <span>{children}</span>
    </p>
  );
}

function formatarRelogio(segundos: number) {
  const m = Math.floor(segundos / 60);
  const s = segundos % 60;
  return `${m}:${String(s).padStart(2, "0")}`;
}
