import type { Metadata } from "next";
import { PaginaLegal, Secao } from "@/app/legal/conteudo";
import { SmartLink } from "@/components/ui/SmartLink";
import { links, site } from "@/lib/site";

export const metadata: Metadata = {
  title: "Privacidade",
  description: "O que o Otimiza coleta, o que sai da sua máquina (e o que só sai se você ligar) e o que acontece com os dados de uma compra.",
  alternates: { canonical: `${site.url}/privacidade/` },
};

export default function PrivacidadePage() {
  return (
    <PaginaLegal
      titulo="Privacidade"
      atualizada="8 de novembro de 2026"
      resumo="A resposta curta: o que o Otimiza mede fica no seu computador. Saem a pergunta de versão, a conferência da licença e o que a área Rede precisa para dizer de quem é cada trecho. A IA do Oti, a comunidade, o backup na nuvem e o Discord só funcionam se você ligar, e vêm desligados. O resto desta página é o detalhe."
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

      <Secao titulo="O que o programa envia para fora, sempre">
        <ul className="list-disc space-y-1.5 pl-5">
          <li>
            <strong>&quot;Saiu versão nova?&quot;</strong>, à API pública do GitHub. Anônima, não leva nada da sua máquina.
          </li>
          <li>
            <strong>A conferência da licença</strong>, à nossa API, a cada poucas semanas: a chave, o código do
            computador, a versão e a impressão (SHA-256) do executável, para saber se é o programa que publicamos. Nós
            guardamos a impressão da chave, não a chave, e uma impressão do endereço IP para notar a mesma chave em
            muitos lugares.
          </li>
          <li>
            <strong>Na área Rede, quando você mede uma rota:</strong> os endereços públicos dos roteadores do caminho vão
            ao IPinfo, que diz de qual operadora é cada um. São endereços da internet, não do seu computador.
          </li>
          <li>
            <strong>As capas dos jogos</strong> vêm do servidor de imagens da Steam, pelo número público de cada jogo.
          </li>
        </ul>
        <p>
          Tudo o que o Otimiza mede — quadros, temperatura, uso de processador, rotas, o que você aplicou e desfez —{" "}
          <strong>fica no seu computador</strong>, em arquivos da sua conta de usuário, a não ser que você ligue uma das
          opções abaixo.
        </p>
      </Secao>

      <Secao titulo="O que só sai se você ligar (tudo vem desligado)">
        <ul className="list-disc space-y-1.5 pl-5">
          <li>
            <strong>Oti com IA</strong> — a pergunta que as respostas prontas não entendem vai à nossa API, com os números
            do seu PC (processador, placa, memória, Hz, FPS da última partida, avisos) e as últimas trocas da conversa. A
            nossa API repassa à API do Claude, da Anthropic, para responder. Nada de arquivo, nome de usuário ou caminho
            de pasta. Guardamos só quantas perguntas a licença fez no dia, para o limite.
          </li>
          <li>
            <strong>Comunidade do Otimiza</strong> — depois de cada partida medida: o nome do processador e da placa, o
            jogo e o FPS; e o plano que ganhou numa calibração. Fica com a impressão da licença, nunca a chave. Nada é
            mostrado com menos de 3 computadores iguais. Desligar apaga o que você mandou.
          </li>
          <li>
            <strong>Backup na nuvem</strong> — quando você aperta &quot;Guardar&quot;: o histórico de partidas e de rede e as
            suas preferências, guardados pela impressão da sua licença, até 256 KB.
          </li>
          <li>
            <strong>Mostrar o jogo no Discord</strong> — o nome do jogo aberto vai ao Discord do seu próprio computador,
            que mostra no seu perfil. Não passa por nós.
          </li>
          <li>
            <strong>Integração com o CS2</strong> — o próprio jogo manda o momento da partida ao Otimiza, dentro do seu
            computador. Não sai dele.
          </li>
        </ul>
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
          <strong>Nós não vemos dado de pagamento.</strong> O Pix e o cartão são processados pelo Mercado Pago. No
          cartão, o número, a validade e o código de segurança são digitados em campos do próprio Mercado Pago, dentro
          desta página, e nunca chegam ao site, ao programa ou aos nossos servidores.
        </p>
        <p>
          <strong>No cartão, o Mercado Pago exige o seu e-mail e o seu CPF.</strong> Eles passam pelo nosso serviço só
          para serem entregues a ele, no momento do pagamento: não são gravados no nosso banco de dados nem escritos
          em registro.
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
            <strong>Mercado Pago</strong> — processa o Pix e o cartão e trata os dados da transação sob a política
            dele, inclusive a prevenção a fraude.
          </li>
          <li>
            <strong>Anthropic</strong> — responde as perguntas do Oti com IA, quando ela está ligada, sob a política
            dela.
          </li>
          <li>
            <strong>IPinfo</strong> — diz de qual operadora é cada roteador de uma rota medida.
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
