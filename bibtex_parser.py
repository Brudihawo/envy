"""parsing for bibtex entries"""
from dataclasses import dataclass

@dataclass
class Error:
    msg: str

    def is_err(self):
        return True

@dataclass
class Entry:
    tipe: str
    name: str
    fields: dict[str, str]

    def is_err(self):
        return False


class Parser:
    def __init__(self, input: str):
        self.data = input
        self.cursor = 0

    def advance(self):
        self.cursor += 1

    def skip_whitespace(self):
        while self.cursor < len(self.data) and self.data[self.cursor].isspace():
            self.advance()

    def peek(self):
        return self.data[self.cursor]

    def consume(self):
        c = self.data[self.cursor]
        self.cursor += 1
        return c

    def parse_type(self) -> str | Error:
        if self.consume() != "@":
            return Error(f"Invalid starting character. Needs to be '@'. Context: {self.get_context()}")

        type_start = self.cursor
        while self.consume() != "{":
            if self.cursor >= len(self.data):
                return Error(f"Nonterminated entry type. Context: {self.get_context()}")

        return self.data[type_start : self.cursor - 1]

    def parse_name(self) -> str | Error:
        name_start = self.cursor
        while self.consume() != ",":
            if self.cursor >= len(self.data):
                return Error(f"Nonterminated name, expected ','. Context {self.get_context()}")

        return self.data[name_start : self.cursor - 1]

    def parse_key(self) -> str | Error:
        name_start = self.cursor
        while self.consume() != "=":
            if self.cursor >= len(self.data):
                return Error(f"Unterminated key, expected '='. Context: {self.get_context()}")

        return self.data[name_start : self.cursor - 1]

    def get_context(self) -> str:
        context_size = 5
        lower_context = self.cursor - context_size
        lower_context = lower_context if lower_context > 0 else 0
        upper_context = self.cursor + context_size
        lower_context = upper_context if upper_context < len(self.data) else len(self.data)
        return self.data[lower_context:upper_context]


    def parse_value(self) -> str | Error:
        start = self.cursor
        depth = 0
        while self.cursor < len(self.data):
            c = self.consume()
            if c == '{':
                depth += 1
            elif c == '}':
                depth -= 1
                if depth < 0:
                    # invalid curly braces
                    return Error(f"Excessive closed curly brace. Context: {self.get_context()}")
            if depth == 0:
                if c == '}':
                    self.skip_whitespace()
                    return self.data[start:self.cursor].strip()
                else:
                    pass

        return Error(f"Unterminated value. Context: {self.get_context()}")


    def parse(self) -> Entry | Error:
        self.skip_whitespace()
        tipe = self.parse_type()
        if isinstance(tipe, Error):
            return tipe
        self.skip_whitespace()

        name = self.parse_name()
        if isinstance(name, Error):
            return name
        name = name.strip()

        data = {}
        while self.cursor < len(self.data):
            self.skip_whitespace()
            c = self.peek()
            if c == "}":
                # end of input
                return Error(f"Premature end of input. Context: {self.get_context()}")

            if not c.isalpha():
                # invalid name
                return Error(f"Invalid key. Expected alphabetic characters. Context: {self.get_context()}")

            key = self.parse_key()
            if isinstance(key, Error):
                return key
            key = key.strip()

            self.skip_whitespace()
            value = self.parse_value()
            if isinstance(value, Error):
                return value
            value = value.strip()
            data[key] = value

            if self.peek() == ',':
                self.consume()
                self.skip_whitespace()
                if self.peek() == '}':
                    return Entry(tipe, name, data)
            else:
                self.skip_whitespace()
                if self.peek() == '}':
                    return Entry(tipe, name, data)

        return Error(f"Unterminated entry. Context: {self.get_context()}")

