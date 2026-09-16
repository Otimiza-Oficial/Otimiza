"use client";

import { ArrowUpRight, Check, Copy } from "lucide-react";
import { useId, useState } from "react";
import { BrandLogo } from "@/components/brand/BrandLogo";
import { CabecalhoPagina, Cartao } from "@/components/painel/AppShell";
import { Button, ButtonLink } from "@/components/ui/Button";
import { useCodigoDaMaquina } from "@/lib/armazem";
import { codigoValido } from "@/lib/licenca";
import { links } from "@/lib/site";

/*
 * Cada situação diz o que acontece (o mesmo que o site e o app dizem) e, quando
 * precisa de gente, entrega a mensagem pronta para colar no Discord.
 */
const SITUACOES = [
  {
    id: "formatei",
    titulo: "Formatei o Windows",
    texto:
      "A chave continua valendo: o código da máquina vem do número de série da placa-mãe, que sobrevive à formatação. Reinstale o Otimiza e cole a mesma chave. Em algumas máquinas o fabricante deixa esse número em branco e o código muda — aí a reemissão é gratuita.",
    mensagem: null,
  },
  {
    id: "placa",
    titulo: "Troquei a placa-mãe ou o computador",
    texto: "O código da máquina mudou e a chave antiga para de valer. Mande o código novo e recebe outra chave, sem custo.",
    mensagem: (codigo: string) =>
      `Oi! Troquei a placa-mãe e preciso reemitir a minha chave do Otimiza.\nCódigo novo desta máquina: ${codigo}`,
  },
  {
    id: "nao-abre",
    titulo: "A chave não abre",
    texto:
      "Confira primeiro em Licenças: ela diz se faltou um pedaço na cópia ou se a chave é de outro computador. Se ainda assim não abrir, mande o código da máquina.",
    mensagem: (codigo: string) =>
      `Oi! A minha chave do Otimiza não está abrindo.\nCódigo desta máquina: ${codigo}`,
  },
  {
    id: "comprar",
    titulo: "Quero comprar",
    texto: "Instale o Otimiza, copie o código da máquina e mande no Discord. A chave é emitida para esse código.",
    mensagem: (codigo: string) => `Oi! Quero comprar a licença do Otimiza.\nCódigo desta máquina: ${codigo}`,
  },
];

export function Suporte() {
  const [codigo, setCodigo] = useCodigoDaMaquina();
  const [copiada, setCopiada] = useState<string | null>(null);
  const idCodigo = useId();
  const valido = codigoValido(codigo);
  const codigoParaMensagem = valido ? codigo : "OTZ-XXXX-XXXX-XXXX";

  const copiar = async (id: string, texto: string) => {
    try {
      await navigator.clipboard.writeText(texto);
      setCopiada(id);
      setTimeout(() => setCopiada((atual) => (atual === id ? null : atual)), 1800);
    } catch {}
  };

  return (
    <>
      <CabecalhoPagina
        titulo="Suporte"
        texto="O atendimento é no Discord, por uma pessoa. Escolha a situação, copie a mensagem pronta e cole no canal."
        acoes={
          <ButtonLink href={links.discord} size="md">
            <BrandLogo brand="discord" size={16} decorative />
            Abrir o Discord
            <ArrowUpRight size={15} strokeWidth={2} aria-hidden="true" />
          </ButtonLink>
        }
      />

      <Cartao className="p-3.5 sm:p-4">
        <label htmlFor={idCodigo} className="text-[12.5px] font-semibold">
          Código deste computador
        </label>
        <input
          id={idCodigo}
          value={codigo}
          onChange={(e) => setCodigo(e.target.value)}
          placeholder="OTZ-XXXX-XXXX-XXXX"
          autoComplete="off"
          spellCheck={false}
          aria-invalid={codigo.length > 0 && !valido}
          className="campo mt-1.5 max-w-[360px] font-mono uppercase"
        />
        <p className="mt-1.5 text-[12px] text-subtle">
          {codigo.length > 0 && !valido
            ? "Isso não tem a forma de um código do Otimiza. Copie de novo direto da tela do programa."
            : "Entra sozinho nas mensagens abaixo. Fica guardado só neste navegador."}
        </p>
      </Cartao>

      <div className="mt-2 grid gap-4 md:grid-cols-2">
        {SITUACOES.map((s) => {
          const mensagem = s.mensagem?.(codigoParaMensagem);
          return (
            <Cartao key={s.id} className="flex flex-col p-3.5 sm:p-4">
              <h2 className="font-display text-[17px] font-semibold tracking-[-0.03em]">{s.titulo}</h2>
              <p className="mt-2 text-[13.5px] leading-[1.6] text-muted">{s.texto}</p>
              {mensagem && (
                <>
                  <pre className="mt-4 rounded-[10px] bg-[#f4f4f3] p-3 font-mono text-[12px] leading-[1.6] whitespace-pre-wrap text-[#2b2b2b]">
                    {mensagem}
                  </pre>
                  <div className="mt-4 flex flex-wrap gap-2">
                    <Button variant="secondary" onClick={() => copiar(s.id, mensagem)} disabled={!valido}>
                      {copiada === s.id ? (
                        <Check size={14} strokeWidth={2.5} aria-hidden="true" />
                      ) : (
                        <Copy size={14} strokeWidth={2} aria-hidden="true" />
                      )}
                      {copiada === s.id ? "Copiada" : "Copiar mensagem"}
                    </Button>
                    {!valido && (
                      <span className="self-center text-[12px] text-subtle">Informe o código acima para copiar.</span>
                    )}
                  </div>
                </>
              )}
            </Cartao>
          );
        })}
      </div>
    </>
  );
}
