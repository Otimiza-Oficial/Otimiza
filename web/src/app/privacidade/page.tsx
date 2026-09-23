import type { Metadata } from "next";
import { PaginaLegal, Secao } from "@/app/legal/conteudo";
import { SmartLink } from "@/components/ui/SmartLink";
import { links, site } from "@/lib/site";

export const metadata: Metadata = {
  title: "Privacidade",
  description: "O que o Otimiza coleta (quase nada), o que sai da sua máquina (uma pergunta de versão) e o que acontece com os dados de uma compra.",
  alternates: { canonical: `${site.url}/privacidade/` },
};

export default function PrivacidadePage() {
  return (
    <PaginaLegal
      titulo="Privacidade"
      atualizada="23 de setembro de 2026"
      resumo="A resposta curta: o programa não manda nada seu para lugar nenhum. Sai uma pergunta de versão, anônima. O resto desta página é o detalhe disso, e o que muda quando você compra."
    >
      <Secao titulo="Quem responde pelos dados">
        <p>
          O controlador é <strong>Eduardo Maciel Wanka</strong>, CPF 100.492.689-88. Para exercer qualquer direito desta
          página, escreva para{" "}
          <a href="mailto:otimizasupport@gmail.com" className="font-medium text-fg underline underline-offset-2">
            otimizasupport@gmail.com
          </a>{" "}
          ou fale no{" "}
          <SmartLink href={links.discord} className="font-medium text-fg underline underline-offset-2">
            Discord
          </SmartLink>
          .
        </p>
      </Secao>

      <Secao titulo="O que o programa envia para fora">
        <p>
          <strong>Uma pergunta, e só:</strong> &quot;saiu versão nova?&quot;, feita à API pública do GitHub. Ela é
          anônima, não leva nada da sua máquina, e sem resposta o programa apenas não avisa e segue funcionando.
        </p>
        <p>
          Não existe telemetria para desligar nas opções porque não existe telemetria. Tudo o que o Otimiza mede —
          quadros, temperatura, uso de processador, o que você aplicou e desfez — <strong>fica no seu computador</strong>,
          em arquivos da sua própria conta de usuário. Nós não temos servidor para onde isso iria.
        </p>
      </Secao>

      <Secao titulo="O código do computador">
        <p>
          O código que aparece na tela de ativação (no formato <span className="font-mono">OTZ-XXXX-XXXX-XXXX</span>) é
          derivado do número de série da placa-mãe. Ele identifica a máquina, não você: não carrega nome, e-mail, nem
          nada que você tenha digitado. Ele existe para a chave valer naquele computador e não em outro.
        </p>
      </Secao>

      <Secao titulo="Quando você compra">
        <p>Para emitir a chave, ficam registrados:</p>
        <ul className="list-disc space-y-1.5 pl-5">
          <li>o código do computador para o qual a chave foi emitida;</li>
          <li>a data da compra e o identificador do pagamento gerado pelo provedor;</li>
          <li>o seu usuário do Discord, quando a compra ou o suporte acontece por lá.</li>
        </ul>
        <p>
          <strong>Nós não vemos dado de pagamento.</strong> A cobrança por Pix é processada pelo provedor de pagamento;
          o site e o programa nunca recebem número de cartão, chave Pix sua ou dado bancário.
        </p>
        <p>
          Esses registros são guardados enquanto a licença existir — ela é vitalícia, e é o que permite reemitir a sua
          chave de graça quando você formata ou troca de peça — e pelo prazo que a lei fiscal exigir do vendedor.
        </p>
      </Secao>

      <Secao titulo="A área do cliente, neste site">
        <p>
          Não há conta nem senha. Entrar é colar a sua chave, e a conferência acontece <strong>dentro do seu navegador</strong>,
          com a mesma chave pública que o programa usa. A chave colada fica guardada apenas neste navegador, e sair
          apaga. Nada disso é enviado para nós.
        </p>
        <p>
          O site não usa cookie de rastreamento, não tem pixel de anúncio e não mede audiência. O que ele guarda no
          navegador é o que você mesmo colou, mais preferências de tela.
        </p>
      </Secao>

      <Secao titulo="Serviços de terceiros que aparecem no caminho">
        <ul className="list-disc space-y-1.5 pl-5">
          <li>
            <strong>GitHub</strong> — hospeda este site e o instalador, e responde a pergunta de versão. Como qualquer
            servidor, ele registra o acesso (endereço IP e navegador).
          </li>
          <li>
            <strong>Discord</strong> — onde acontecem a compra e o suporte. O que você escreve lá está sujeito à
            política do Discord.
          </li>
          <li>
            <strong>Provedor de pagamento</strong> — processa o Pix e trata os dados da transação sob a política dele.
          </li>
        </ul>
      </Secao>

      <Secao titulo="Seus direitos">
        <p>
          Pela LGPD, você pode pedir confirmação do tratamento, acesso, correção, anonimização, portabilidade e exclusão
          dos dados, além de revogar consentimento. Peça pelo contato acima.
        </p>
        <p>
          <strong>Uma ressalva honesta:</strong> apagar o registro da sua compra apaga também a prova de que a licença é
          sua — depois disso não temos como reemitir a chave sem cobrar de novo. Nós dizemos isso antes de apagar, para
          a escolha ser informada.
        </p>
      </Secao>

      <Secao titulo="Relatório de diagnóstico">
        <p>
          O programa gera, quando você pede, um relatório com hardware, versões e o que foi alterado, para você levar ao
          suporte. Ele é um arquivo no seu computador: só sai da sua máquina se você mesmo enviar. Ele não contém senha,
          token nem arquivos pessoais.
        </p>
      </Secao>
    </PaginaLegal>
  );
}
