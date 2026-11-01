# Ícone do Auli — tinta e violeta

Cores: "A" e pingo do "i" em violeta `#6D3BEF`; "u", "l" e haste do "i" em tinta `#1B1A2E`; fundo `#FFFFFF`.
Versão escura: letras `#F1F0F7`, violeta clareado `#8B63F5` (mantém o contraste), fundo `#1B1A2E`.
Traço: 11 unidades no desenho-base de 144 × 86.

## Arquivos

| Arquivo | Uso |
|---|---|
| `auli-icon.svg` | Ícone mestre, branco, cantos arredondados e contorno finíssimo (favicon SVG, site) |
| `auli-icon-escuro.svg` | Variante para fundos escuros |
| `auli-icon-fullbleed.svg` | Quadrado cheio, letras na zona segura (Android, PWA maskable, iOS) |
| `auli-wordmark-fundo-claro.svg` / `-fundo-escuro.svg` | Logotipo sem fundo, para cabeçalho |
| `favicon.ico` | 16, 32 e 48 px |
| `png/auli-icon-{16,32,48,192,512}.png` | Rasters do ícone arredondado |
| `png/auli-icon-escuro-512.png` | Raster da variante escura |
| `png/apple-touch-icon.png` | 180 px, quadrado cheio (o iOS arredonda) |
| `png/auli-icon-maskable-512.png` | PWA `purpose: maskable` |
| `png/auli-icon-1024.png` | Fonte para o gerador de ícones do Tauri |

## Web (`index.html`)

```html
<link rel="icon" href="/favicon.ico" sizes="any">
<link rel="icon" href="/auli-icon.svg" type="image/svg+xml">
<link rel="apple-touch-icon" href="/apple-touch-icon.png">
<meta name="theme-color" content="#FFFFFF">
```

## Manifesto PWA

```json
"background_color": "#FFFFFF",
"icons": [
  { "src": "/auli-icon-192.png", "sizes": "192x192", "type": "image/png" },
  { "src": "/auli-icon-512.png", "sizes": "512x512", "type": "image/png" },
  { "src": "/auli-icon-maskable-512.png", "sizes": "512x512", "type": "image/png", "purpose": "maskable" }
]
```

## Tauri (Android/desktop)

```sh
cargo tauri icon png/auli-icon-1024.png
```
