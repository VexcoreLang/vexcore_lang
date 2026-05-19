# ─────────────────────────────────────────────
#  PenLang :: Runner
#  python -m penlang script.vcl
#  python -m penlang --repl
# ─────────────────────────────────────────────
import sys
import os
from core.lexer import Lexer, LexerError
from core.parser import Parser, ParseError
from core.interpreter import Interpreter, RuntimeError_


def run_source(source: str, debug: bool = False):
    try:
        tokens      = Lexer(source).tokenize()
        if debug:
            print("=== TOKENS ===")
            for t in tokens: print(" ", t)
        program     = Parser(tokens).parse()
        if debug:
            print("\n=== AST ===")
            print(program)
        interp = Interpreter()
        interp.run(program)
    except LexerError as e:
        print(f"\n{e}", file=sys.stderr)
        sys.exit(1)
    except ParseError as e:
        print(f"\n{e}", file=sys.stderr)
        sys.exit(1)
    except RuntimeError_ as e:
        print(f"\n{e}", file=sys.stderr)
        sys.exit(1)


def run_file(path: str, debug: bool = False):
    if not os.path.exists(path):
        print(f"[PenLang] File not found: {path}", file=sys.stderr)
        sys.exit(1)
    with open(path, encoding="utf-8") as f:
        source = f.read()
    run_source(source, debug=debug)


def repl():
    print("PenLang REPL  (ctrl+c или exit; для выхода)")
    print("Многострочный ввод: заканчивай блок пустой строкой\n")
    buf = []
    while True:
        try:
            prompt = "... " if buf else ">>> "
            line   = input(prompt)
            if line.strip() in ("exit", "quit"):
                break
            buf.append(line)
            # запускаем когда строка заканчивается на ; или блок закрыт
            src = "\n".join(buf)
            if src.strip().endswith(";") or (src.count("{") > 0 and src.count("{") == src.count("}")):
                # оборачиваем в scenary если нет секции
                if "[scenary]" not in src:
                    src = "[scenary]\n" + src
                run_source(src)
                buf = []
        except KeyboardInterrupt:
            print()
            break
        except EOFError:
            break


def main():
    args   = sys.argv[1:]
    debug  = "--debug" in args
    args   = [a for a in args if not a.startswith("--")]

    if not args or args[0] == "--repl":
        repl()
    else:
        run_file(args[0], debug=debug)


if __name__ == "__main__":
    main()
