import type { Metadata } from "next";
import { PaginaLegal, REVISAR, Secao } from "@/app/legal/conteudo";
import { SmartLink } from "@/components/ui/SmartLink";
import { formatarReais, links, PRECO_BRL, site } from "@/lib/site";

export const metadata: Metadata = {
  title: "Termos de uso",
  description: "As condições de venda e uso da licença do Otimiza: o que a chave dá, como ela é reemitida e como pedir o dinheiro de volta.",
  alternates: { canonical: `${site.url}/termos/` },
};

export default function TermosPage() {
  return (
    <PaginaLegal
      titulo="Termos de uso"
      atualizada="23 de setembro de 2026"
      resumo="O que você compra, o que o programa faz na sua máquina e o que acontece quando algo dá errado. Escrito para ser lido, não para ser aceito sem ler."
    >
      <Secao titulo="Quem vende">
        <p>
          O Otimiza é vendido por {REVISAR} (nome empresarial), inscrito sob {REVISAR} (CNPJ ou CPF), com contato em{" "}
          {REVISAR} (e-mail) e atendimento no{" "}
          <SmartLink href={links.discord} className="font-medium text-fg underline underline-offset-2">
            Discord
          </SmartLink>
          .
        </p>
      </Secao>

      <Secao titulo="O que você compra">
        <p>
          O programa é <strong>gratuito para baixar, instalar e medir</strong>. A licença de {formatarReais(PRECO_BRL)},
          paga uma única vez, libera as otimizações que escrevem no Windows.
        </p>
        <p>
          A licença é <strong>vitalícia e presa a um computador</strong>: ela é uma assinatura digital emitida para o
          código daquela máquina. Não há mensalidade, não há renovação, e as atualizações do produto não são cobradas de
          novo. Para um segundo computador é preciso uma segunda licença.
        </p>
      </Secao>

      <Secao titulo="Formatar, trocar peça, reinstalar">
        <p>
          Formatar o Windows não invalida a sua licença. Trocar a placa-mãe muda o código da máquina — nesse caso a
          chave é <strong>reemitida sem custo</strong>, pelo Discord. Esse é o mesmo canal de quem perdeu a chave: ela é
          sua, e reenviá-la não é um novo produto.
        </p>
      </Secao>

      <Secao titulo="Devolução">
        <p>
          Compra feita pela internet tem <strong>sete dias de arrependimento</strong>, contados do pagamento, conforme o
          artigo 49 do Código de Defesa do Consumidor. Peça pelo Discord ou pelo e-mail de contato e o valor é devolvido
          integralmente, sem você precisar justificar.
        </p>
        <p>
          Passados os sete dias, a devolução continua acontecendo quando o produto não entrega o que promete na sua
          máquina — por exemplo, quando o programa não abre, ou quando a chave não ativa e o suporte não consegue
          resolver.
        </p>
      </Secao>

      <Secao titulo="O que o programa faz, e o que ele não faz">
        <p>
          Toda alteração que o Otimiza faz no Windows guarda o valor anterior e pode ser desfeita dentro do próprio
          programa. A lista completa do que ele altera está na tela, com o risco e o modo de desfazer de cada item.
        </p>
        <p>
          Ele <strong>não</strong> injeta nada em jogo, não modifica arquivo de jogo sem prévia e sem cópia, não desliga
          antivírus, firewall ou atualização do Windows, e não promete número que não tenha sido medido na sua máquina.
        </p>
        <p>
          <strong>Sobre o ganho:</strong> o resultado depende do seu hardware, do jogo e do que já está configurado. O
          produto mede antes e depois e mostra o número — inclusive quando ele diz que não mudou nada, e inclusive
          desfazendo sozinho o que piorou. Nenhuma porcentagem de ganho é prometida antes da medição.
        </p>
      </Secao>

      <Secao titulo="Uso aceitável">
        <p>
          A chave é para o seu uso. Revender, publicar ou distribuir a chave a terceiros encerra a licença — é o único
          caso em que uma chave é desativada.
        </p>
      </Secao>

      <Secao titulo="Limites">
        <p>
          O Otimiza funciona no Windows 10 e 11, 64 bits. Alterações feitas por você fora do programa, imagens
          modificadas do Windows (as chamadas &quot;lite&quot;) e falhas de hardware estão fora do que ele consegue
          responder — e, quando detecta esses casos, ele avisa em vez de aplicar por cima.
        </p>
        <p>
          Nenhum software elimina a possibilidade de defeito. Por isso o produto grava o estado anterior de cada
          alteração e oferece o desfazer — e por isso a devolução existe.
        </p>
      </Secao>

      <Secao titulo="Mudanças nestes termos">
        <p>
          Se estas condições mudarem, a data acima muda junto e a versão anterior continua valendo para quem já comprou.
          Nada aqui é alterado com efeito retroativo.
        </p>
      </Secao>
    </PaginaLegal>
  );
}
