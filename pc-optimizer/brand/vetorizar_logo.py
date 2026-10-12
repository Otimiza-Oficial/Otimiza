"""
Vetoriza a logo do Otimiza a partir do arquivo enviado pelo dono (`brand/logo-fonte.png`, branco sobre transparente)
e gera tudo o que usa a marca: os SVGs do app e do site e o ícone 1024 de onde saem os ícones do Windows
(`npx tauri icon brand/icone-app-1024.png`).

O contorno é lido da transparência ampliada, simplificado e convertido em curvas (Catmull-Rom para Bézier), sem
redesenhar a forma.

Uso: python brand/vetorizar_logo.py
"""

import os

import cv2
import numpy as np

AQUI = os.path.dirname(os.path.abspath(__file__))
RAIZ = os.path.dirname(AQUI)
REPO = os.path.dirname(RAIZ)

AMPLIAR = 8           # a fonte tem 512 px: o contorno é tirado em 4096
PASSO = 28            # distância entre pontos do contorno, em pixels da imagem ampliada (3,5 px da fonte)
SUAVIZAR = 1.5        # desvio da média gaussiana, em pontos
MARGEM = 0.04         # folga em volta da forma, em fração do lado
FUNDO_DO_ICONE = (11, 11, 11)   # o preto da marca
RAIO_DO_ICONE = 0.22  # cantos do quadrado do ícone, em fração do lado
MARCA_NO_ICONE = 0.70  # quanto do ícone a forma ocupa


def contornos():
    fonte = cv2.imread(os.path.join(AQUI, "logo-fonte.png"), cv2.IMREAD_UNCHANGED)
    alfa = fonte[:, :, 3]
    grande = cv2.resize(alfa, None, fx=AMPLIAR, fy=AMPLIAR, interpolation=cv2.INTER_CUBIC)
    grande = cv2.GaussianBlur(grande, (0, 0), AMPLIAR * 0.6)
    _, mascara = cv2.threshold(grande, 127, 255, cv2.THRESH_BINARY)
    achados, _ = cv2.findContours(mascara, cv2.RETR_CCOMP, cv2.CHAIN_APPROX_NONE)
    suaves = [suavizar(c[:, 0, :].astype(float)) / AMPLIAR for c in achados if len(c) >= 16]
    return suaves, fonte.shape[1]


def suavizar(c):
    """Pontos a cada PASSO px pelo contorno e média gaussiana circular: sem isso a curva tinha quinas onde o
    contorno em escada da imagem mudava de direção."""
    segmentos = np.linalg.norm(np.diff(np.vstack([c, c[:1]]), axis=0), axis=1)
    acumulado = np.concatenate([[0], np.cumsum(segmentos)])
    total = acumulado[-1]
    fechado = np.vstack([c, c[:1]])
    alvos = np.arange(0, total, PASSO)
    pts = np.column_stack([np.interp(alvos, acumulado, fechado[:, k]) for k in range(2)])
    n = len(pts)
    raio = int(3 * SUAVIZAR)
    pesos = np.exp(-0.5 * (np.arange(-raio, raio + 1) / SUAVIZAR) ** 2)
    pesos /= pesos.sum()
    estendido = np.vstack([pts[-raio:], pts, pts[:raio]])
    return np.column_stack(
        [np.convolve(estendido[:, k], pesos, mode="valid")[:n] for k in range(2)]
    )


def caixa(cs):
    pts = np.vstack(cs)
    x0, y0 = pts.min(axis=0)
    x1, y1 = pts.max(axis=0)
    lado = max(x1 - x0, y1 - y0)
    lado_total = lado * (1 + 2 * MARGEM)
    cx, cy = (x0 + x1) / 2, (y0 + y1) / 2
    return cx - lado_total / 2, cy - lado_total / 2, lado_total


def caminho(cs, ox, oy, escala):
    partes = []
    for c in cs:
        p = (c - [ox, oy]) * escala
        n = len(p)
        d = [f"M{p[0][0]:.2f} {p[0][1]:.2f}"]
        for i in range(n):
            p0, p1, p2, p3 = p[(i - 1) % n], p[i], p[(i + 1) % n], p[(i + 2) % n]
            c1 = p1 + (p2 - p0) / 6
            c2 = p2 - (p3 - p1) / 6
            d.append(f"C{c1[0]:.2f} {c1[1]:.2f} {c2[0]:.2f} {c2[1]:.2f} {p2[0]:.2f} {p2[1]:.2f}")
        partes.append("".join(d) + "Z")
    return "".join(partes)


def amostrar(cs, ox, oy, escala, passos=6):
    """Os mesmos Béziers do SVG, em polígonos densos, para rasterizar o ícone igual ao vetor."""
    poligonos = []
    for c in cs:
        p = (c - [ox, oy]) * escala
        n = len(p)
        pts = []
        for i in range(n):
            p0, p1, p2, p3 = p[(i - 1) % n], p[i], p[(i + 1) % n], p[(i + 2) % n]
            c1 = p1 + (p2 - p0) / 6
            c2 = p2 - (p3 - p1) / 6
            for t in np.linspace(0, 1, passos, endpoint=False):
                u = 1 - t
                pts.append(u**3 * p1 + 3 * u * u * t * c1 + 3 * u * t * t * c2 + t**3 * p2)
        poligonos.append(np.array(pts))
    return poligonos


def svg(d, cor, rotulo=True):
    aria = ' role="img" aria-label="Otimiza"' if rotulo else ""
    return (
        f'<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 512 512"{aria}>'
        f'<path fill="{cor}" fill-rule="evenodd" d="{d}"/></svg>\n'
    )


def icone(cs, ox, oy, lado, tamanho=1024, super_=4):
    g = tamanho * super_
    img = np.zeros((g, g, 4), np.uint8)
    # Quadrado de cantos redondos no preto da marca: a forma é branca e sumiria numa barra de tarefas clara.
    raio = int(g * RAIO_DO_ICONE)
    fundo = np.zeros((g, g), np.uint8)
    cv2.rectangle(fundo, (raio, 0), (g - raio, g), 255, -1)
    cv2.rectangle(fundo, (0, raio), (g, g - raio), 255, -1)
    for cx, cy in [(raio, raio), (g - raio, raio), (raio, g - raio), (g - raio, g - raio)]:
        cv2.circle(fundo, (cx, cy), raio, 255, -1, cv2.LINE_AA)
    img[:, :, 0], img[:, :, 1], img[:, :, 2] = FUNDO_DO_ICONE[2], FUNDO_DO_ICONE[1], FUNDO_DO_ICONE[0]
    img[:, :, 3] = fundo
    # A forma ocupa MARCA_NO_ICONE do lado, centrada.
    escala = g * MARCA_NO_ICONE / lado
    deslocar = g * (1 - MARCA_NO_ICONE) / 2
    marca = np.zeros((g, g), np.uint8)
    polis = [np.round(pp + deslocar).astype(np.int32) for pp in amostrar(cs, ox, oy, escala)]
    cv2.fillPoly(marca, polis, 255, cv2.LINE_AA)
    peso = marca.astype(np.float32) / 255
    for canal, tinta in enumerate((240, 243, 245)):  # BGR do branco quente da marca
        img[:, :, canal] = (img[:, :, canal] * (1 - peso) + tinta * peso).astype(np.uint8)
    final = cv2.resize(img, (tamanho, tamanho), interpolation=cv2.INTER_AREA)
    cv2.imwrite(os.path.join(AQUI, "icone-app-1024.png"), final)


def gravar(caminho_rel, texto):
    destino = os.path.join(REPO, caminho_rel)
    with open(destino, "w", encoding="utf-8", newline="\n") as f:
        f.write(texto)
    print("gravado", caminho_rel)


def main():
    cs, _ = contornos()
    ox, oy, lado = caixa(cs)
    d = caminho(cs, ox, oy, 512 / lado)
    marca = svg(d, "currentColor")
    for rel in [
        "pc-optimizer/brand/logo.svg",
        "pc-optimizer/brand/otimiza-marca.svg",
        "pc-optimizer/public/logo.svg",
        "pc-optimizer/src/assets/otimiza-marca.svg",
        "web/public/brand/otimiza-mark.svg",
    ]:
        gravar(rel, marca)
    # O favicon do site não herda cor: a forma escura, como o anterior.
    gravar("web/src/app/icon.svg", svg(d, "#0a0a0a", rotulo=False))
    icone(cs, ox, oy, lado)
    print("gravado pc-optimizer/brand/icone-app-1024.png;", sum(len(c) for c in cs), "pontos em", len(cs), "contornos")


if __name__ == "__main__":
    main()
