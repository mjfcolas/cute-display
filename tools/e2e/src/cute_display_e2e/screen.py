"""What the glass says, as the console's `describe` gives it: lines `name value`, ` *`
on the one chosen, the first `front <app>`."""
from dataclasses import dataclass

CHOSEN = ' *'


@dataclass(frozen=True)
class Screen:
    lines: tuple[str, ...]

    def front(self):
        return self.first_value('front')

    def first_value(self, name):
        return next(iter(self.values(name)), None)

    def values(self, name):
        """Each line called `name`, without its name and its mark."""
        prefix = f'{name} '
        return [line.removeprefix(prefix).removesuffix(CHOSEN) for line in self.lines if line.startswith(prefix)]

    def chosen(self):
        return next((line.removesuffix(CHOSEN) for line in self.lines if line.endswith(CHOSEN)), None)

    def __str__(self):
        return '\n'.join(self.lines)
