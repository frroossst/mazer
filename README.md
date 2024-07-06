# MAPLE 🍁
## Maple is Another Pesky LaTeX Emulator

A minimal, simple math markup language that compiles to HTML, written in Rust.  

It's like LaTeX, but with a very stripped down feature set, mainly intended for note taking and simple math expressions.


# Motivations

- **Simple:** Natural Language alternative to math markup like LaTeX or MathML  

- **Opinionated:** This is what works for me, your preferences may be different  

- **Portable:** Plain text files that compile to HTML  

- **Verbose:** Typing speed is not a concern which enables expressive and flexible syntax  
  
- **Minimal:** Limited features (more to be added as I need them)  


# Installation

> cargo install maple


# Usage

> maple <path to file>
Outputs a HTML file of the same name

> maple <path to file> --serve
Starts a server to serve HTML file and watch for file changes
 
> maple <path to file> --serve --open
Starts a server and opens the page in the default web browser (open only works in conjunction with serve)

> maple <path to file> --dry-run
Runs the maple compiler without creating the HTML file, useful for debugging!
