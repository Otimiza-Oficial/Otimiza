"use client";

import { Check, KeyRound } from "lucide-react";
import { useId, useState, type FormEvent } from "react";
import { CabecalhoPagina, Cartao } from "@/components/painel/AppShell";
import { CartaoLicenca } from "@/components/painel/Licenca";
import { Button } from "@/components/ui/Button";
import { useCodigoDaMaquina, useLicencas } from "@/lib/armazem";
import { codigoValido, conferir, explicar } from "@/lib/licenca";

export function Licencas() {
  const { licencas, adicionar, remover } = useLicencas();
  const [codigo, setCodigo] = useCodigoDaMaquina();
  const [chave, setChave] = useState("");
  const [mensagem, setMensagem] = useState<{ tipo: "ok" | "erro"; texto: string } | null>(null);
  const [conferindo, setConferindo] = useState(false);
  const idCodigo = useId();
  const idChave = useId();

  const codigoPreenchido = codigo.trim().length > 0;
  const codigoOk = !codigoPreenchido || codigoValido(codigo);

  const aoEnviar = async (e: FormEvent) => {
    e.preventDefault();
    if (!chave.trim() || !codigoOk) return;
    setConferindo(true);
    const r = await conferir(chave, codigoPreenchido ? codigo : null);
    setConferindo(false);
    if (!r.ok) {
      setMensagem({ tipo: "erro", texto: explicar(r.recusa) });
      return;
    }
    const nova = adicionar(chave);
    setMensagem({
      tipo: "ok",
      texto: nova ? "A chave confere e foi guardada neste navegador." : "Esta chave já está guardada aqui.",
    });
    setChave("");
  };

  return (
    <>
      <CabecalhoPagina
        titulo="Licenças"
        texto="Cole a sua chave para conferir para qual computador ela foi emitida e se continua valendo. A conferência usa a mesma chave pública do programa, aqui no seu navegador."
      />

      <div className="grid items-start gap-5 lg:grid-cols-[minmax(0,1fr)_minmax(0,1.15fr)]">
        <Cartao className="p-3.5 sm:p-4">
          <h2 className="font-display text-[18px] font-semibold tracking-[-0.03em]">Conferir uma chave</h2>
          <form onSubmit={aoEnviar} className="mt-2 space-y-4" noValidate>
            <div>
              <label htmlFor={idCodigo} className="text-[12.5px] font-semibold">
                Código deste computador <span className="font-normal text-subtle">(opcional)</span>
              </label>
              <input
                id={idCodigo}
                value={codigo}
                onChange={(e) => setCodigo(e.target.value)}
                placeholder="OTZ-XXXX-XXXX-XXXX"
                autoComplete="off"
                spellCheck={false}
                aria-invalid={!codigoOk}
                aria-describedby={`${idCodigo}-ajuda`}
                className="campo mt-1.5 font-mono uppercase"
              />
              <p id={`${idCodigo}-ajuda`} className="mt-1.5 text-[12px] leading-[1.5] text-subtle">
                {codigoOk
                  ? "Aparece no Otimiza, na tela de ativação. Com ele, a conferência diz se a chave é deste PC."
                  : "Isso não tem a forma de um código do Otimiza: OTZ- seguido de três blocos de quatro caracteres."}
              </p>
            </div>

            <div>
              <label htmlFor={idChave} className="text-[12.5px] font-semibold">
                A sua chave
              </label>
              <textarea
                id={idChave}
                value={chave}
                onChange={(e) => {
                  setChave(e.target.value);
                  setMensagem(null);
                }}
                rows={4}
                spellCheck={false}
                placeholder="Cole aqui a chave inteira, do começo ao fim"
                className="campo mt-1.5 h-auto resize-none py-2.5 font-mono text-[12.5px] leading-[1.5] break-all"
              />
            </div>

            <Button type="submit" size="md" className="w-full" disabled={!chave.trim() || !codigoOk || conferindo}>
              {conferindo ? "Conferindo…" : "Conferir e guardar"}
            </Button>

            {mensagem && (
              <p
                role="status"
                className={
                  mensagem.tipo === "ok"
                    ? "flex items-start gap-2 rounded-[10px] bg-ink p-3 text-[13px] leading-[1.5] text-white"
                    : "rounded-[10px] bg-[#f1f1f0] p-3 text-[13px] leading-[1.5] text-[#2b2b2b] shadow-[inset_0_0_0_1px_rgb(10_10_10/0.08)]"
                }
              >
                {mensagem.tipo === "ok" && <Check size={15} strokeWidth={2.5} className="mt-0.5 shrink-0" aria-hidden="true" />}
                {mensagem.texto}
              </p>
            )}
          </form>
        </Cartao>

        <div className="space-y-4">
          <div className="flex items-baseline justify-between">
            <h2 className="font-display text-[18px] font-semibold tracking-[-0.03em]">Neste navegador</h2>
            <p className="text-[12.5px] text-muted">
              {licencas.length === 1 ? "1 chave" : `${licencas.length} chaves`}
            </p>
          </div>

          {licencas.length === 0 ? (
            <Cartao className="grid place-items-center px-6 py-14 text-center">
              <span className="grid size-12 place-items-center rounded-[12px] bg-[#f1f1f0]">
                <KeyRound size={20} strokeWidth={2} aria-hidden="true" />
              </span>
              <p className="font-display mt-4 text-[16px] font-semibold tracking-[-0.02em]">Nenhuma chave guardada</p>
              <p className="mt-1.5 max-w-[320px] text-[13px] leading-[1.55] text-muted">
                Confira a sua chave ao lado. Ela fica guardada só aqui, para você ter à mão quando formatar ou trocar
                de computador.
              </p>
            </Cartao>
          ) : (
            licencas.map((l) => (
              <CartaoLicenca
                key={l.chave}
                chave={l.chave}
                adicionadaEm={l.adicionadaEm}
                maquina={codigoPreenchido && codigoOk ? codigo : null}
                onRemover={() => remover(l.chave)}
              />
            ))
          )}
        </div>
      </div>
    </>
  );
}
