# HULK Compiler

HULK (Havana University Language for Kompilers) es un lenguaje de programación **orientado a objetos, orientado a expresiones y con tipado estático**, desarrollado para la asignatura Compilación de la Universidad de La Habana.

Este repositorio contiene una implementación completa de un compilador para HULK escrita en **Rust**, capaz de traducir programas del lenguaje a **LLVM IR**, el cual posteriormente puede compilarse a código nativo mediante LLVM y Clang.

## 📖 Especificación del lenguaje

La definición completa de la sintaxis y semántica de HULK se encuentra en:

https://matcom.github.io/hulk/appendix-hulk-syntax.html


## Características implementadas

| Funcionalidad | Estado |
|--------|:------:|
| Expresiones y operadores | ✅ Completo |
| Funciones (inline y bloque) | ✅ Completo |
| Variables y asignación destructiva | ✅ Completo |
| Condicionales (`if` / `elif` / `else`) | ✅ Completo |
| Bucles (`while`, `for`) | ✅ Completo |
| Tipos nominales, herencia y polimorfismo | ✅ Completo |
| Verificación de tipos y conformancia | ✅ Completo |
| Inferencia de tipos | ✅ Completo |

### Extensiones implementadas

- **Polimorfismo paramétrico en funciones**.
- **Protocolos**.



## 🏗️ Arquitectura del compilador

El compilador sigue una arquitectura clásica organizada en varias etapas:

1. **Análisis léxico** → Tokenización mediante un analizador basado en DFA.
2. **Análisis sintáctico** → Construcción del AST utilizando LALRPOP.
3. **Análisis semántico** → Inferencia y comprobación de tipos, verificación de protocolos y resolución de la jerarquía de herencia.
4. **Monomorfización** → Especialización de funciones genéricas para los tipos concretos utilizados.
5. **Generación de código** → Traducción del AST a LLVM IR mediante Inkwell.



## 📦 Requisitos

- Rust (versión estable)
- LLVM 17
- Clang 17


## 🚀 Uso

### 1. Compilar el proyecto

El proyecto puede compilarse utilizando el `Makefile` incluido:

```bash
make build
```

Este comando genera el ejecutable `hulk` en la raíz del repositorio.

### Compilar un programa HULK

Una vez construido el compilador, un programa HULK puede compilarse ejecutando:

```bash
./hulk programa.hulk
```

Durante la compilación se ejecutan automáticamente todas las fases del compilador. Si el programa no contiene errores, se generan dos archivos en el directorio actual:

- `output.ll`: código LLVM IR generado por el compilador.
- `output`: ejecutable nativo producido automáticamente mediante `clang-17`.

### Ejecutar el programa generado

Una vez finalizada la compilación, el ejecutable puede ejecutarse con:

```bash
./output
```

### 2. Utilizar Cargo directamente

No es necesario construir previamente el ejecutable `hulk`. Puede ejecutarse el compilador directamente con Cargo:

```bash
cargo run -- programa.hulk
```

Este comando realiza exactamente el mismo proceso de compilación que `./hulk`, generando igualmente los archivos `output.ll` y `output`.

### 3. Modo de desarrollo

El proyecto incluye un modo de depuración pensado para inspeccionar las representaciones internas del compilador.

Puede ejecutarse mediante:

```bash
cargo run --features dev -- programa.hulk
```

En este modo, además de realizar la compilación completa y generar el ejecutable, es posible imprimir información intermedia para facilitar el desarrollo y la depuración, incluyendo:

- AST después del análisis sintáctico.
- AST anotado con los tipos inferidos.
- AST tras el pase de monomorfización.

Estas opciones pueden activarse o desactivarse desde `main.rs` al invocar `compiler_dev::compile`.

## 🧪 Pruebas

El directorio `tests/` contiene programas que ejercitan todas las características implementadas del lenguaje.

## 📚 Documentación

El informe completo del proyecto se encuentra en:

```text
docs/informe.pdf
```

Este documento recoge el análisis crítico del diseño del lenguaje HULK, la descripción detallada de las extensiones implementadas, los fundamentos teóricos de cada fase del compilador y la arquitectura del mismo. 

## 📄 Licencia

Este proyecto fue desarrollado con fines docentes para la asignatura **Compilación** de la Universidad de La Habana.
