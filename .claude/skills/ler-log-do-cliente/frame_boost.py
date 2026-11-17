"""Resumo do Frame Boost a partir do otimiza.log (ou do Otimiza-log-<data>.txt do botão "Salvar em Downloads").

Uso:  python -I frame_boost.py <log> [<log.1> ...]

Para cada sessão do gerador: versão, placa e Hz, modo, e as medianas das janelas de 10 s com o jogo na frente
(janelas com a sobreposição escondida em 50% ou mais ficam de fora). No fim, o veredito de cada promessa da 1.0.8:
fila de 1 quadro sem perder gerados, tela lisa, atraso e o FPS do jogo (a guarda).

Lê as linhas da 1.0.7 também (sem os campos novos, que saem como "—"): dá para comparar antes e depois.
"""

import re
import statistics
import sys

DATA = re.compile(r"^\[(\d{4}-\d\d-\d\d \d\d:\d\d:\d\d)\.\d+\] \[(\w+)\] (.*)$")
NUM = r"(-?[\d.]+)"


def campo(padrao, texto, conv=float):
    m = re.search(padrao, texto)
    return conv(m.group(1)) if m else None


def mediana(v):
    v = [x for x in v if x is not None]
    return statistics.median(v) if v else None


def fmt(v, casas=1, sufixo=""):
    return "—" if v is None else f"{v:.{casas}f}{sufixo}"


class Sessao:
    def __init__(self, inicio, versao):
        self.inicio = inicio
        self.fim = inicio
        self.versao = versao
        self.placa = None
        self.hz = None
        self.modo = None
        self.janelas = []  # dicts das linhas de 10 s
        self.telas = []  # dicts das linhas "tela medida"
        self.razoes = []
        self.avisos = []

    def nova_janela(self, t):
        j = {
            "reais": campo(rf"· {NUM} reais/s", t),
            "gerados": campo(rf"· {NUM} gerados/s", t),
            "escondida": campo(rf"escondida {NUM}%", t),
            "mult": campo(r"· (\d+)× ·", t, int),
            "placa_pct": campo(rf"\({NUM}% do tempo\)", t),
            "repetidas_antigo": campo(rf"{NUM}% das atualizações repetidas", t),
            "descartados": campo(rf"· {NUM} descartados/s", t),
            "sem_vez": campo(r"\((\d+) sem vez na fila\)", t, int),
            "ate_present": campo(rf"atraso até o Present {NUM} ms", t),
            "fila": campo(rf"fila da sobreposição até a tela {NUM} ms", t),
        }
        self.janelas.append(j)

    def validas(self):
        return [j for j in self.janelas if (j["escondida"] or 0) < 50]


def ler(caminhos):
    linhas = []
    for c in caminhos:
        with open(c, encoding="utf-8", errors="replace") as f:
            linhas.extend(f.read().splitlines())
    sessoes, atual, versao = [], None, None
    for linha in linhas:
        m = DATA.match(linha)
        if not m:
            continue
        quando, nivel, t = m.groups()
        if t.startswith("Versão do Otimiza"):
            versao = t.split()[-1]
            continue
        if not t.startswith("gerador"):
            continue
        # Começo de sessão: a linha da placa ("gerador: <placa> em \\.\DISPLAY1 (144 Hz), 1920x1080").
        hz = re.match(r"gerador: (.+) em (\S+) \((\d+) Hz", t)
        if hz:
            atual = Sessao(quando, versao)
            atual.placa, atual.hz = hz.group(1), int(hz.group(3))
            sessoes.append(atual)
            continue
        if atual is None:
            continue
        atual.fim = quando
        if t.startswith("gerador: modo "):
            atual.modo = t[len("gerador: modo "):]
        elif t.startswith("gerador: 10 s"):
            atual.nova_janela(t)
        elif t.startswith("gerador: tela medida ·") and "quadros novos" in t:
            atual.telas.append({
                "novos": campo(rf"· {NUM} quadros novos/s", t),
                "repetidas": campo(rf"· {NUM}% das atualizações repetidas", t),
                "engasgos": campo(r"· (\d+) engasgos", t, int),
            })
        elif "da guarda" in t and "manteve" in t:
            atual.razoes.append(campo(rf"manteve {NUM}% do FPS real", t))
        elif nivel in ("WARN", "ERROR") or "desligado pela guarda" in t or "modo leve" in t:
            atual.avisos.append(f"{quando} {t}")
    return sessoes


def veredito(s):
    v = s.validas()
    out = []
    reais = mediana(j["reais"] for j in v)
    gerados = mediana(j["gerados"] for j in v)
    mult = mediana(j["mult"] for j in v)
    sem_vez = sum(j["sem_vez"] or 0 for j in v)
    total_gerados = sum((j["gerados"] or 0) * 10 for j in v)
    if any(j["sem_vez"] is not None for j in v) and total_gerados > 0:
        frac = sem_vez / (total_gerados + sem_vez) * 100
        estado = "OK" if frac <= 2 else ("ATENÇÃO" if frac <= 5 else "PROBLEMA")
        out.append(f"[{estado}] Fila de 1 quadro: {frac:.1f}% dos gerados perderam a vez (meta ≤ 2%).")
    else:
        out.append("[—] Fila de 1 quadro: sem a contagem nova (versão antes da 1.0.8 ou modo Auto).")
    if s.telas:
        rep = mediana(t["repetidas"] for t in s.telas)
        novos = mediana(t["novos"] for t in s.telas)
        eng = sum(t["engasgos"] or 0 for t in s.telas)
        minutos = len(s.telas) * 10 / 60
        esperado = min(s.hz or 0, (reais or 0) * (mult or 1)) if reais else None
        falta = (1 - novos / esperado) * 100 if esperado and novos else None
        estado = "OK" if (falta is not None and falta <= 5 and eng / max(minutos, 0.1) <= 2) else "ATENÇÃO"
        out.append(
            f"[{estado}] Tela: {fmt(novos, 0)} quadros novos/s de {fmt(esperado, 0)} esperados "
            f"({fmt(falta, 1)}% abaixo), {fmt(rep)}% repetidas, {eng} engasgos em {minutos:.1f} min."
        )
    else:
        out.append("[—] Tela medida: sem as linhas (sem PresentMon/administrador, ou versão antes da 1.0.8).")
    ate = mediana(j["ate_present"] for j in v)
    fila = mediana(j["fila"] for j in v)
    if ate is not None:
        total = ate + (fila or 0)
        periodo = 1000 / s.hz if s.hz else None
        out.append(
            f"[INFO] Atraso acrescentado: {total:.1f} ms (até o Present {ate:.1f} + fila até a tela {fmt(fila)})"
            + (f" = {total / periodo:.1f} atualizações do monitor." if periodo else ".")
            + (" Fila acima de 2 atualizações: a fila de 1 não está valendo." if fila and periodo and fila > 2 * periodo else "")
        )
    razoes = [r for r in s.razoes if r is not None]
    if razoes:
        m = statistics.mean(razoes)
        estado = "OK" if m >= 90 else ("ATENÇÃO" if m >= 80 else "PROBLEMA")
        out.append(f"[{estado}] FPS do jogo com o Frame Boost: {m:.0f}% do sem ele ({len(razoes)} comparações da guarda).")
    return out


def main():
    sys.stdout.reconfigure(encoding="utf-8")
    if len(sys.argv) < 2:
        print(__doc__)
        sys.exit(1)
    sessoes = [s for s in ler(sys.argv[1:]) if s.janelas]
    if not sessoes:
        print("Nenhuma sessão do Frame Boost com linhas de 10 s no log.")
        return
    for i, s in enumerate(sessoes, 1):
        v = s.validas()
        print(f"\n=== Sessão {i}: {s.inicio} → {s.fim} · versão {s.versao or '?'} ===")
        print(f"Placa: {s.placa} · {s.hz} Hz · modo: {s.modo or '— (linha de modo só a partir da 1.0.8)'}")
        print(f"Janelas de 10 s: {len(s.janelas)} ({len(v)} com o jogo na frente)")
        if not v:
            print("Todas com a sobreposição escondida (jogo atrás ou sem quadros): nada para medir.")
            continue
        print(
            "Medianas: "
            f"{fmt(mediana(j['reais'] for j in v), 0)} reais/s · {fmt(mediana(j['gerados'] for j in v), 0)} gerados/s · "
            f"{fmt(mediana(j['descartados'] for j in v), 0)} descartados/s · placa {fmt(mediana(j['placa_pct'] for j in v))}% do tempo"
        )
        for linha in veredito(s):
            print("  " + linha)
        if s.avisos:
            print(f"  Avisos ({len(s.avisos)}):")
            for a in s.avisos[:12]:
                print("   - " + a[:220])


if __name__ == "__main__":
    main()
