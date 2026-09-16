"use client";

import { ArrowRight, Copy, Download, House, KeyRound, LifeBuoy, LogOut, Search, Settings } from "lucide-react";
import { useRouter } from "next/navigation";
import { useEffect, useMemo, useRef, useState } from "react";
import { BrandLogo } from "@/components/brand/BrandLogo";
import { limparTudo, useLicencas } from "@/lib/armazem";
import { cn } from "@/lib/cn";
import { conferir } from "@/lib/licenca";
import { links } from "@/lib/site";

/*
 * A BUSCA PROCURA O QUE EXISTE, E SÓ.
 *
 * Ela varre as seções do painel, as ações que a tela sabe executar e as chaves
 * guardadas neste navegador — inclusive pelo código da máquina, que é como o
 * cliente chama o PC dele. Não há índice remoto: não há servidor.
 */
type Resultado = {
  id: string;
  titulo: string;
  detalhe?: string;
  grupo: "Ir para" | "Ações" | "Suas licenças";
  icone: React.ReactNode;
  executar: () => void;
};

export function Busca({ aberta, aoFechar }: { aberta: boolean; aoFechar: () => void }) {
  const [termo, setTermo] = useState("");
  const [indice, setIndice] = useState(0);
  const { licencas } = useLicencas();
  const [maquinas, setMaquinas] = useState<{ chave: string; maquina: string }[]>([]);
  const campo = useRef<HTMLInputElement>(null);
  const router = useRouter();

  // As chaves guardadas viram resultado pelo código da máquina — conferindo de
  // novo, porque o que está no armazenamento não é confiado.
  useEffect(() => {
    let vivo = true;
    (async () => {
      const lidas: { chave: string; maquina: string }[] = [];
      for (const l of licencas) {
        const r = await conferir(l.chave, null);
        if (r.ok) lidas.push({ chave: l.chave, maquina: r.dados.maquina });
      }
      if (vivo) setMaquinas(lidas);
    })();
    return () => {
      vivo = false;
    };
  }, [licencas]);

  // A caixa é remontada a cada abertura (`key` no AppShell), então o campo já
  // nasce vazio: aqui só resta pôr o cursor nele.
  useEffect(() => {
    if (!aberta) return;
    const t = setTimeout(() => campo.current?.focus(), 20);
    return () => clearTimeout(t);
  }, [aberta]);

  const ir = (href: string) => () => {
    aoFechar();
    router.push(href);
  };
  const abrir = (href: string) => () => {
    aoFechar();
    window.open(href, "_blank", "noopener,noreferrer");
  };

  const todos = useMemo<Resultado[]>(() => {
    const icone = (n: React.ReactNode) => <span className="text-muted">{n}</span>;
    const base: Resultado[] = [
      { id: "inicio", grupo: "Ir para", titulo: "Início", icone: icone(<House size={14} strokeWidth={2} />), executar: ir("/painel/") },
      { id: "licencas", grupo: "Ir para", titulo: "Licenças", detalhe: "Conferir e guardar uma chave", icone: icone(<KeyRound size={14} strokeWidth={2} />), executar: ir("/painel/licencas/") },
      { id: "downloads", grupo: "Ir para", titulo: "Downloads", detalhe: "Instalador da versão mais nova", icone: icone(<Download size={14} strokeWidth={2} />), executar: ir("/painel/downloads/") },
      { id: "suporte", grupo: "Ir para", titulo: "Suporte", detalhe: "Mensagens prontas para o Discord", icone: icone(<LifeBuoy size={14} strokeWidth={2} />), executar: ir("/painel/suporte/") },
      { id: "conta", grupo: "Ir para", titulo: "Preferências", detalhe: "O que este navegador guardou", icone: icone(<Settings size={14} strokeWidth={2} />), executar: ir("/painel/conta/") },
      { id: "baixar", grupo: "Ações", titulo: "Baixar o Otimiza", detalhe: "Windows 10 e 11, 64 bits", icone: icone(<Download size={14} strokeWidth={2} />), executar: abrir(links.baixar) },
      { id: "discord", grupo: "Ações", titulo: "Abrir o Discord", detalhe: "Compra e suporte", icone: <BrandLogo brand="discord" size={14} decorative />, executar: abrir(links.discord) },
      {
        id: "sair",
        grupo: "Ações",
        titulo: "Sair deste navegador",
        detalhe: "Apaga as chaves guardadas aqui",
        icone: icone(<LogOut size={14} strokeWidth={2} />),
        executar: () => {
          limparTudo();
          aoFechar();
        },
      },
    ];

    const chaves: Resultado[] = maquinas.map((m) => ({
      id: `maquina-${m.maquina}`,
      grupo: "Suas licenças",
      titulo: m.maquina,
      detalhe: "Copiar a chave deste computador",
      icone: icone(<Copy size={14} strokeWidth={2} />),
      executar: async () => {
        try {
          await navigator.clipboard.writeText(m.chave);
        } catch {}
        aoFechar();
      },
    }));

    return [...base, ...chaves];
  }, [maquinas, router]); // eslint-disable-line react-hooks/exhaustive-deps

  const achados = useMemo(() => {
    const t = termo.trim().toLowerCase();
    if (!t) return todos;
    return todos.filter((r) => `${r.titulo} ${r.detalhe ?? ""}`.toLowerCase().includes(t));
  }, [termo, todos]);

  if (!aberta) return null;

  const aoTeclar = (e: React.KeyboardEvent) => {
    if (e.key === "ArrowDown") {
      e.preventDefault();
      setIndice((i) => Math.min(i + 1, achados.length - 1));
    } else if (e.key === "ArrowUp") {
      e.preventDefault();
      setIndice((i) => Math.max(i - 1, 0));
    } else if (e.key === "Enter") {
      e.preventDefault();
      achados[indice]?.executar();
    } else if (e.key === "Escape") {
      aoFechar();
    }
  };

  let grupoAtual = "";

  return (
    <div className="fixed inset-0 z-[60] flex items-start justify-center bg-[rgb(10_10_10/0.35)] px-4 pt-[12vh]" onClick={aoFechar}>
      <div
        role="dialog"
        aria-modal="true"
        aria-label="Pesquisar na Otimiza"
        onClick={(e) => e.stopPropagation()}
        onKeyDown={aoTeclar}
        className="w-full max-w-[520px] animate-[surgir_140ms_ease-out] overflow-hidden rounded-[10px] bg-white shadow-[0_0_0_1px_rgb(10_10_10/0.1),0_24px_48px_-16px_rgb(10_10_10/0.35)]"
      >
        <div className="flex items-center gap-2.5 border-b border-line px-3.5">
          <Search size={14} strokeWidth={2} className="shrink-0 text-subtle" aria-hidden="true" />
          <input
            ref={campo}
            value={termo}
            onChange={(e) => {
              setTermo(e.target.value);
              setIndice(0);
            }}
            placeholder="Pesquisar seções, ações e licenças"
            className="h-11 w-full bg-transparent text-[13px] outline-none placeholder:text-subtle"
          />
          <kbd className="rounded-[4px] bg-[#f1f1f0] px-1.5 py-0.5 font-mono text-[10px] text-subtle">Esc</kbd>
        </div>

        <ul className="max-h-[320px] overflow-y-auto p-1.5">
          {achados.length === 0 && (
            <li className="px-2.5 py-6 text-center text-[12.5px] text-muted">Nada encontrado para “{termo}”.</li>
          )}
          {achados.map((r, i) => {
            const cabecalho = r.grupo !== grupoAtual ? ((grupoAtual = r.grupo), r.grupo) : null;
            return (
              <li key={r.id}>
                {cabecalho && (
                  <p className="px-2.5 pt-2.5 pb-1 text-[10px] font-medium tracking-[0.1em] text-subtle uppercase">{cabecalho}</p>
                )}
                <button
                  type="button"
                  onMouseEnter={() => setIndice(i)}
                  onClick={r.executar}
                  className={cn(
                    "flex w-full items-center gap-2.5 rounded-[6px] px-2.5 py-2 text-left text-[12.5px]",
                    i === indice ? "bg-[#f1f1f0]" : "hover:bg-[#f6f6f5]",
                  )}
                >
                  {r.icone}
                  <span className={cn("font-medium", r.grupo === "Suas licenças" && "font-mono text-[12px]")}>{r.titulo}</span>
                  {r.detalhe && <span className="truncate text-[11.5px] text-subtle">{r.detalhe}</span>}
                  <ArrowRight size={13} strokeWidth={2} className={cn("ml-auto shrink-0 text-subtle", i !== indice && "opacity-0")} aria-hidden="true" />
                </button>
              </li>
            );
          })}
        </ul>
      </div>
    </div>
  );
}
