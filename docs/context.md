# Ofertas Rosario: contexto del proyecto

Referencia técnica del proyecto: stack, reglas y cómo se lee cada fuente. Dónde estamos, qué falta y las decisiones tomadas están en `STATUS.md` (raíz del repo). El brief (`docs/brief.md`) explica el porqué.

Última actualización: 11/11/2026.

## Stack

| Parte | Tecnología |
| --- | --- |
| Scrapers, detector y generación del sitio | Rust 2024 (`tokio`, `reqwest` 0.13, `scraper`, `serde`, `regex`, `chrono`, `toml`, `minijinja` para las plantillas HTML) |
| TLS | rustls para todo, salvo DAR: su IIS 8.5 solo ofrece `ECDHE-RSA-AES256-SHA384` (CBC), que rustls no implementa. DAR usa OpenSSL compilado dentro del binario (`native-tls-vendored`), sin depender de paquetes del sistema. |
| Frontend | HTML generado por Rust + JS plano, sin framework ni bundler |
| PWA | `manifest.webmanifest` + `sw.js` |
| Hosting | Cloudflare Workers con static assets, `wrangler deploy` |
| Corrida diaria | GitHub Actions con cron: scrapea, genera `dist/`, hace `wrangler deploy` con token de API y commitea `data/` |
| Tests | `cargo test` con respuestas de muestra guardadas |

Entorno de desarrollo: Rust estable (probado con 1.99.1) en Linux; en Windows, WSL. Los comandos están en `STATUS.md`.

## Formato de los datos

- `data/offers/<cadena>.json`: un array con un objeto `Offer` por línea, ordenado por `id`. Así el commit diario solo muestra cambios reales de precio.
- Los precios van en **centavos** (`price`, `list_price`), enteros, para evitar errores de redondeo.
- Campos de `Offer`: `chain`, `store`, `id` (id propio de la cadena), `ean`, `name`, `price`, `list_price`, `discount_pct`, `promo`, `url`, `image`, `source` (`web`).
- `data/run.json`: resumen de la última corrida por cadena (`ok`, `offers`, `seconds`, `error`). Si una cadena falla (error o 0 ofertas), su archivo de ofertas del día anterior queda como está en `data/`, pero el sitio la publica vacía con un aviso (ver abajo).
- El detector descarta `list_price` menores o iguales al precio, o más de 5 veces mayores (caso Jumbo), y calcula `discount_pct`.

## Sitio generado (`dist/`)

`cargo run --release -- build` lee `data/`, `config/stores.toml` y `config/site.toml`, y escribe:

| Ruta | Qué es |
| --- | --- |
| `/` | Portada: búsqueda, guardados, las 20 mejores ofertas de cada cadena. |
| `/<cadena>/`, `/<cadena>/pagina/N/` | Todas las ofertas de la cadena, de a 100, ordenadas por descuento. |
| `/datos/` | Explicación de los datos abiertos y sus campos. |
| `/datos/<cadena>.json`, `/datos/hoy.csv` | Datos abiertos. |
| `/preguntas-frecuentes/`, `/terminos/`, `/privacidad/` | Páginas informativas y legales. Muestran `contact` de `config/site.toml` si no está vacío. |
| `/datos/buscar.json` | Índice compacto para la búsqueda: `[clave, cadena, nombre, precio, precio_anterior, descuento, promo, url]`. |
| `/sitemap.xml`, `/robots.txt`, `/llms.txt` | SEO y descubrimiento por IAs. |
| `/manifest.webmanifest`, `/sw.js`, `/app.js`, `/icons/` | PWA. |
| `/_headers` | Headers de Cloudflare: `sw.js` sin caché, `datos/` con CORS abierto y `max-age=0, must-revalidate` (el HTML ya sale así por defecto). Para todo: CSP solo `'self'` (nada de terceros ni inline), `frame-ancestors 'none'`, Permissions-Policy, COOP y HSTS. Si se suma un recurso externo o un estilo inline, hay que tocar la CSP. |
| `/404.html` | Página de error (Workers la usa con `not_found_handling: "404-page"`). |

Si `run.json` marca una cadena con `ok: false`, el build la publica sin ofertas: su sección y la portada dicen "Hoy no pudimos leer las ofertas de ...", y queda fuera de `buscar.json` y `hoy.csv` (`datos/<cadena>.json` sale como `[]`). Plantilla del aviso: `templates/failed.html`.

Las plantillas (`templates/`) y los assets (`web/`) se embeben en el binario con `include_str!`/`include_bytes!`: hay que recompilar para ver cambios. La clave de cada oferta es `<cadena>:<id>`; los guardados se guardan en `localStorage` con esa clave.

## SEO y descubrimiento por IAs

- HTML generado de antemano, con las ofertas escritas en la página. Nada de SPA con `<div id="root">` vacío.
- Una sección por cadena (la página de descuentos bancarios está pendiente, ver `STATUS.md`). Cada página con `title`, `description`, canonical, Open Graph y `lang="es-AR"`.
- JSON-LD de schema.org (`ItemList` + `Offer`, precio en ARS).
- `sitemap.xml` con fecha diaria y `robots.txt` que permite GPTBot, ClaudeBot, PerplexityBot y Google-Extended. Revisar que Cloudflare no tenga activado el bloqueo de bots de IA.
- `llms.txt` con el resumen del proyecto y links a los datos.
- Datos abiertos: `/datos/<cadena>.json` y `/datos/hoy.csv`.
- Al publicar: Google Search Console y Bing Webmaster (Bing alimenta ChatGPT Search y Copilot).

## Reglas de cortesía con los sitios

- User-Agent identificable, en `config/stores.toml`. Hoy es `OfertasRosario/0.1 (proyecto open source sin fines de lucro)`; al lanzar pasa a `OfertasRosario/0.1 (+<url del repo>)`. Nunca hacerse pasar por un navegador.
- Respetar `robots.txt`.
- Una corrida por día, pausa entre requests, sin paralelismo agresivo.
- No extraer keys ni tokens de las páginas para usar sus APIs privadas.

## Fuentes verificadas (10 y 11/11/2026)

### Carrefour (VTEX Intelligent Search)

- Endpoint: `GET https://www.carrefour.com.ar/api/io/_v/api/intelligent-search/product_search/category-1/<slug>`
- Parámetros: `query=`, `count=50`, `page=N`, `sort=discount:desc`, `hideUnavailableItems=true`, `regionId=U1cjY2FycmVmb3VyYXIwMDE4` (base64 de `SW#carrefourar0018`, Rosario).
- La región cambia el precio: yerba Mañanita 1 kg a $4.589 con región y $5.749 sin región.
- Categorías de súper (de `/api/catalog_system/pub/category/tree/1`): `almacen`, `desayuno-y-merienda`, `bebidas`, `lacteos-y-productos-frescos`, `carnes-y-pescados`, `frutas-y-verduras`, `panaderia`, `congelados`, `limpieza`, `perfumeria-y-farmacia`, `mundo-bebe`, `mascotas`.
- Por producto: `productName`, `items[0].ean`, `items[0].sellers[0].commertialOffer.{Price, ListPrice, AvailableQuantity}`. Descuento = 1 − Price / ListPrice.
- Se pagina hasta que el descuento llega a 0. Sin `hideUnavailableItems` aparecen primero productos sin stock con precio 0.
- Límite: más allá de ~2.500 resultados por consulta la API no devuelve nada (página 60 de 50 vino vacía). Por eso se consulta por categoría.
- `Teasers` (promos tipo 2x1) vienen vacíos en esta API. Queda pendiente.
- No hace falta el hash de GraphQL que usa ratoneando (rota cada pocos meses).
- Primera corrida real (10/11/2026): 1.643 ofertas en 12 categorías, 172 s. Todas con EAN y precio de lista. Descuento mediano 14%, máximo 70%.

### Coto (Endeca, endpoint viejo)

- La API nueva (`api.coto.com.ar/.../products/offers/...`) exige una `key` de Constructor.io. Descartada.
- Endpoint: `GET https://www.coto.com.ar/sitios/cdigi/categoria/_/N-<tipoOferta>?Nrpp=50&No=<offset>&Nr=<filtro>&format=json`
- Filtro de tienda: `Nr=AND(product.sDisp_165:1004,product.language:español,OR(product.siteId:CotoDigital))`. Con 165 da menos resultados que con 200 (3.023 contra 4.071 en "Hasta 30% DTO"), o sea que filtra por disponibilidad en Alto Rosario.
- Tipos de oferta (`product.tipoOferta`): la dimensión es multi-selección, así que todos los tipos se combinan en una sola consulta `N-aZbZc...` (OR). El scraper los descubre en cada corrida: lee el menú de la dimensión en `/sitios/cdigi/categoria?Nr=<filtro>&format=json` y sigue su `moreLink` para ver todos. El 10/11/2026 había 22 tipos (2X1, 3X2, 6X5, 8X7, "Hasta 20/30/40/50% DTO!!", "2da Unid 50/70%", "Exclusivas", etc.).
- `Nrpp` acepta al menos 500 (3,6 MB en ~1 s). Se usa 200.
- Primera corrida real (10/11/2026): 13.400 ofertas disponibles en la 165, 99 s. Descuento mediano 30%, máximo 80%. Promos más comunes: "30%Dto", "25%Dto", "35%Dto", "1 Pago 20%".
- Categorías (dimensión `product.category`): Almacén, Bebidas, Frescos, Congelados, Limpieza, Perfumería, Electro, Textil, Hogar, Aire Libre. El scraper descarta los productos cuyo `allAncestors.displayName` incluye alguna de `exclude_categories` (hoy Electro, Textil, Hogar y Aire Libre). Con el filtro quedaron 4.766 ofertas (se descartaron 8.634).
- URL de producto: se arma como `https://www.coto.com.ar/sitios/cdigi/productos` + `detailsAction.recordState` (sin `?format=json`). Redirige a `/productos/...` del sitio nuevo y muestra el producto. Verificado el 11/11/2026: porotos Lonquimay a $1.201,80 con 70% off y precio regular $4.006, igual que en nuestros datos.
- Promos de 2da unidad: `precioDesc` es el precio por unidad llevando 2, y `textoLlevando` dice la condición ("Llevando 2"). El scraper arma `promo` como "70% 2da (Llevando 2)".
- Por producto (en `records[0].attributes`): `product.displayName`, `product.eanPrincipal`, `sku.dtoPrice` (JSON con `precioLista`), `product.dtoDescuentos` (JSON con `textoDescuento` y `precioDescuento`), `product.dtoDescuentosMediosPago`, `product.sDisp_<tienda>`.
- Precio normal = `precioLista`. Si no es mayor que el de oferta, se usa `textoPrecioRegular` de `product.dtoDescuentos` ("Precio Contado: $2499"), que es lo que la web muestra como "Precio regular". Pasa en "Otras Ofertas" (verdulería, pescadería, carnicería): ahí `precioLista` es igual al precio de oferta. Cuando están los dos, coinciden. Verificado el 11/11/2026 en la web (Naranja Jugo $899 con regular $2.499, Lechuga Capuchina, Merluza). Precio de oferta = `precioDescuento`. El campo `precio` de `sku.dtoPrice` NO es el precio del producto (parece precio por unidad de medida).
- El precio es de la lista 200 (online). Según el brief, la 165 coincidió con la 200 en 150 ofertas comparadas. Validar con SEPA más adelante.
- `sDisp`: 1004 parece "disponible" y 1001 "no disponible". A confirmar.

### DAR (ASP clásico)

- No tiene `robots.txt`.
- Sin los headers `Accept: text/html...` y `Accept-Language: es-AR`, el sitio rebota en loop entre `index.asp` y `Login.asp`. Con esos headers (y el User-Agent identificable) responde normal. No hace falta login.
- TLS viejo: solo ofrece `ECDHE-RSA-AES256-SHA384`. Con rustls la conexión se corta ("connection reset by peer"); con OpenSSL anda.
- Flujo con una misma sesión (cookie `ASPSESSIONID...`):
  1. `GET /index.asp` siguiendo la redirección a `Login.asp`, que sirve la home (~15 KB). **Hay que entrar por `index.asp`:** si se entra directo a `Login.asp` devuelve una página de 3 KB y la sesión queda inservible (`Productos.asp` rebota a login).
  2. `POST /carrito.asp` con `TM=CM&1n=undefined&2n=undefined&3n=undefined&4n=undefined&Crit=&Dat=` (abre la sección Ofertas).
  3. `GET /Productos.asp?Of=S` (página 1).
  4. `GET /productos.asp?page=N&N1=&N2=&N3=&N4=` (páginas siguientes).
- ~300 ofertas: 15 páginas de 20. El tamaño de página se cambia con el select `cantProd` (cookie, hasta 50).
- Por producto (`li.cuadProd`): nombre en `.desc`, precio en `.precio` (formato `$475,<b>00</b>`), código de 13 dígitos en `PCompra('0000000029345')` (código interno, no EAN). Foto en `Fotos/Articulos/<codigo>.jpg`.
- Solo trae precio final, sin precio anterior. El descuento real sale del historial propio.
- Primera corrida real (10/11/2026): 280 ofertas en 15 páginas, 17 s.
- Detalle por producto en `Detalle.asp?Pr=<codigo>&P=1` (sin revisar).
- Promos bancarias: banner en la home, sin revisar.

### Descartadas

- **La Gallega**: bloqueada por `robots.txt`.
- **Coto API nueva**: requiere key.
- **Todo Chen**: sin web.

## Referencias

- [ratoneando.ar](https://ratoneando.ar) y [ratoneando-go](https://github.com/matiasbontempo/ratoneando-go): comparador en vivo (Go + Redis), precios nacionales. De ahí sale el endpoint Endeca de Coto. Su web es una SPA sin contenido en el HTML: lo que no hay que hacer para SEO.

## Pendientes

La lista de tareas vive en `STATUS.md`.
