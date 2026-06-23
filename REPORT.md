# HULK Compiler

HULK (Havana University Language for Kompiers) es un lenguaje de programación **orientado a objetos, orientado a expresiones y con tipado estático**, desarrollado para la asignatura Compilación de la Universidad de La Habana.

Este repositorio contiene una implementación completa de un compilador para HULK escrita en **Rust**, capaz de traducir programas del lenguaje a **LLVM IR**, el cual posteriormente puede compilarse a ejecutables nativos mediante LLVM y Clang.


# 📋 Características implementadas

La siguiente tabla resume el grado de implementación de las funcionalidades definidas en la especificación de HULK.

| Funcionalidad | Estado |
|--------------|:------:|
| Expresiones y operadores | ✅ Completo |
| Funciones (inline y bloque) | ✅ Completo |
| Variables y asignación destructiva | ✅ Completo |
| Condicionales (`if` / `elif` / `else`) | ✅ Completo |
| Bucles (`while`, `for`) | ✅ Completo |
| Tipos nominales e herencia | ✅ Completo |
| Verificación de tipos y conformancia | ✅ Completo |
| Inferencia de tipos | ✅ Completo |

#  Extensiones implementadas

Además de cumplir la especificación del lenguaje, el compilador incorpora dos extensiones principales:

### Polimorfismo paramétrico en funciones

Se añadió soporte para **funciones genéricas**, permitiendo que una misma definición pueda reutilizarse con distintos tipos de argumentos.

La genericidad puede escribirse de forma **explícita**, utilizando el parámetro de tipo reservado `T`,

```hulk
function id(x: T): T => x;
```

o de forma **implícita**, omitiendo completamente las anotaciones de tipo,

```hulk
function id(x) => x;
```

Ambas definiciones son equivalentes. Durante el análisis semántico el compilador determina automáticamente cuándo una función es genérica.

Posteriormente, un pase de **monomorfización** genera una copia especializada de la función para cada combinación distinta de tipos utilizada en el programa. Como resultado, el backend recibe un AST completamente monomórfico, por lo que el backend no necesita ningún mecanismo adicional para soportar genericidad.

### Protocolos

Se incorporó soporte para **protocolos**, introduciendo un mecanismo de **tipado estructural** que complementa el sistema nominal de clases de HULK.

Un protocolo define un conjunto de métodos que un tipo debe proporcionar. La implementación es **implícita**: un tipo conforma un protocolo simplemente por poseer los métodos requeridos con las firmas correctas, sin necesidad de declararlo explícitamente.

La verificación se realiza completamente durante el análisis semántico mediante comprobación de conformancia estructural. Una vez finalizada esta fase, toda la información relativa a protocolos desaparece del programa, por lo que el backend no requiere ningún tratamiento especial.


# Arquitectura del compilador

El compilador sigue una arquitectura clásica organizada en varias etapas consecutivas, donde cada una transforma la representación del programa hasta obtener el código LLVM final.

1. **Análisis léxico**  
   Convierte el código fuente en una secuencia de tokens mediante la simulación de un autómata finito determinista (DFA) construido a partir de expresiones regulares, aplicando la estrategia de longest match y complementado con reconocimiento manual para literales numéricos, cadenas e identificadores.

2. **Análisis sintáctico**  
   Comprueba que la secuencia de tokens cumple la gramática del lenguaje y construye el Árbol de Sintaxis Abstracta (AST) mediante un parser LALR(1) implementado con LALRPOP.

3. **Análisis semántico**  
   Realiza la inferencia y comprobación de tipos, verifica la conformancia con protocolos, resuelve la jerarquía de herencia y construye la representación aplanada de los tipos que será utilizada posteriormente durante la generación de código.

4. **Monomorfización**  
   Elimina la genericidad generando versiones especializadas de las funciones genéricas para cada combinación concreta de tipos utilizada en el programa.

5. **Generación de código**  
   Traduce el AST resultante a LLVM IR utilizando la biblioteca Inkwell. Durante esta etapa se generan las estructuras que representan los objetos, las tablas virtuales para el despacho dinámico, las funciones y el punto de entrada del programa.


# 🧪 Compilación y ejecución

## Requisitos

- Rust (estable)
- LLVM 15
- Clang 15

## 1. Compilar el proyecto

Para compilar el proyecto y generar el ejecutable se puede usar make:

```bash
make build
```

Este comando deja el ejecutable hulk en la raíz del proyecto.

## Compilar un programa HULK

Una vez compilado el proyecto, un programa HULK puede compilarse mediante:

```bash
./hulk programa.hulk
```

El compilador ejecuta todas las fases de compilación y, si el programa es correcto, produce el archivo `output.ll`, que contiene el código LLVM IR generado. 

## 2. Usar Cargo directamente

```bash
cargo run -- programa.hulk
```

## Ejecutar el programa generado

```bash
./output
```
El comando ejecuta el programa e imprime resultados.

## 3. Modo de desarrollo

Para facilitar la depuración del compilador se incluye un modo de desarrollo:

```bash
cargo run --features dev -- programa.hulk
```

En este modo pueden visualizarse representaciones intermedias generadas durante la compilación según configuración, como el AST tipado y el resultado del pase de monomorfización, además de ejecutarse automáticamente el código LLVM generado.

# 🧪 Pruebas

El directorio `tests/` contiene programas que ejercitan todas las características implementadas del lenguaje.

# ⚠️ Limitaciones actuales

- No se implementa gestión automática de memoria. Los objetos y cadenas se reservan dinámicamente mediante `malloc` y no son liberados durante la ejecución.
- Los bucles `for` únicamente permiten iterar sobre `range`.
- El polimorfismo paramétrico únicamente está soportado en funciones; no se admiten tipos genéricos.

# 📖 Documentación

El informe completo del proyecto se encuentra en docs/informe.pdf. Este documento recoge el análisis crítico del diseño del lenguaje HULK, la descripción detallada de las extensiones implementadas, los fundamentos teóricos de cada fase del compilador y la arquitectura del mismo.