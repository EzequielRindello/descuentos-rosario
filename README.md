# Ofertas Rosario

Las ofertas de los supermercados de Rosario, todos los días, en un solo lugar.

Todas las mañanas se revisan las webs de **Coto**, **Carrefour** y **DAR** y se juntan las ofertas del día con precio de Rosario: precio de oferta contra precio normal, y promos tipo 2x1 o 70% en la segunda unidad.

- **Sin registrarte:** entrás y mirás.
- **Instalable en el celu** como una app, y funciona sin señal adentro del súper.
- **Solo ofertas de hoy:** si un día no se puede leer la web de una cadena, su sección queda vacía en vez de mostrar precios viejos.
- **Gratis y de código abierto.**

**👉 [ofertas-rosario.rindelloezequiel.workers.dev](https://ofertas-rosario.rindelloezequiel.workers.dev)** (en construcción)

## Instalala en el celu

No hace falta ninguna tienda de apps.

- **Android (Chrome):** entrá a la web y tocá **Instalar** cuando aparezca el aviso. Si no aparece, abrí el menú del navegador (⋮) y tocá **Instalar app** o **Agregar a pantalla principal**.
- **iPhone (Safari):** entrá a la web, tocá **Compartir** (el cuadrado con la flecha) y después **Agregar a inicio**.

Queda un ícono como cualquier app. La última versión que abriste se puede ver sin señal, y cuando hay conexión se actualiza sola.

## Los datos son tuyos también

Las ofertas del día se pueden descargar desde la web, en [`/datos/`](https://ofertas-rosario.rindelloezequiel.workers.dev/datos/): un JSON por cadena y un CSV con todas. También están en este repo, en [`data/offers/`](data/offers/), con el historial de cada día en los commits ([`data/run.json`](data/run.json) dice si alguna cadena no se pudo leer en la última corrida). Los precios están en centavos. Podés usarlos para lo que quieras.

Los precios salen de las webs públicas de cada cadena y pueden no coincidir con el de la góndola.

## Para quien quiera colaborar

- [`STATUS.md`](STATUS.md): en qué estado está el proyecto y qué falta.
- [`docs/context.md`](docs/context.md): cómo se lee cada supermercado.
- [`config/stores.toml`](config/stores.toml): las sucursales. Se puede corregir con un PR sin saber programar.

Licencia [MIT](LICENSE). La tipografía es [Inter](https://github.com/rsms/inter), con licencia SIL Open Font License ([`web/fonts/OFL.txt`](web/fonts/OFL.txt)).
