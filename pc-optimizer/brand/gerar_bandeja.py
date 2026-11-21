# Os ícones da bandeja (src-tauri/icons/bandeja/<humor>.png, 64 × 64): o Oti da marca v2 na versão pequena (a da
# entrega `simbolo-16`, olhos mais grossos para não sumirem a 16 px), branco, com o olhar de cada momento. A geometria
# e os ângulos são os mesmos de `ui/src/componentes/Oti.svelte` (OLHAR). Desenhado em 16× e reduzido, para a borda
# sair lisa. Uso: `python -I brand/gerar_bandeja.py src-tauri/icons/bandeja` (precisa do Pillow).
import math
import sys
from pathlib import Path

from PIL import Image, ImageDraw

OLHAR = {"atento": 0, "feliz": 0, "pensando": -42, "jogando": 32, "preocupado": 78, "dormindo": 128}
# Olhos da versão pequena, no quadro 64 × 64 com o disco de raio 30 no centro: (x, y, largura, altura, centro x, centro y).
OLHOS = [(36.5, 16.1, 9.0, 19.8, 41.0, 26.0), (49.25, 9.8, 7.5, 18.6, 53.0, 19.1)]
DISCO = (255, 255, 255, 255)
OLHO = (0x12, 0x12, 0x1C, 255)
ESCALA = 16


def girar(p, centro, graus):
    a = math.radians(graus)
    x, y = p[0] - centro[0], p[1] - centro[1]
    return (centro[0] + x * math.cos(a) - y * math.sin(a), centro[1] + x * math.sin(a) + y * math.cos(a))


def capsula(x, y, w, h, passos=24):
    r = w / 2
    pontos = []
    for i in range(passos + 1):  # meia-volta de cima
        t = math.pi + math.pi * i / passos
        pontos.append((x + r + r * math.cos(t), y + r + r * math.sin(t)))
    for i in range(passos + 1):  # meia-volta de baixo
        t = math.pi * i / passos
        pontos.append((x + r + r * math.cos(t), y + h - r + r * math.sin(t)))
    return pontos


def desenhar(olhar):
    lado = 64 * ESCALA
    img = Image.new("RGBA", (lado, lado), (0, 0, 0, 0))
    d = ImageDraw.Draw(img)
    c = 32 * ESCALA
    d.ellipse([c - 30 * ESCALA, c - 30 * ESCALA, c + 30 * ESCALA, c + 30 * ESCALA], fill=DISCO)
    olhos = Image.new("RGBA", (lado, lado), (0, 0, 0, 0))
    do = ImageDraw.Draw(olhos)
    for x, y, w, h, cx, cy in OLHOS:
        pts = [girar(p, (cx, cy), -32) for p in capsula(x, y, w, h)]
        pts = [girar(p, (32, 32), olhar) for p in pts]
        do.polygon([(px * ESCALA, py * ESCALA) for px, py in pts], fill=OLHO)
    # O olho de trás é cortado pela borda do disco (raio 29,7), como na entrega.
    mascara = Image.new("L", (lado, lado), 0)
    ImageDraw.Draw(mascara).ellipse([c - 29.7 * ESCALA, c - 29.7 * ESCALA, c + 29.7 * ESCALA, c + 29.7 * ESCALA], fill=255)
    img.paste(olhos, (0, 0), Image.composite(olhos, Image.new("RGBA", (lado, lado)), mascara).split()[3])
    return img.resize((64, 64), Image.LANCZOS)


if __name__ == "__main__":
    pasta = Path(sys.argv[1])
    for humor, graus in OLHAR.items():
        desenhar(graus).save(pasta / f"{humor}.png")
        print(pasta / f"{humor}.png")
