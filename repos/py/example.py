from math import pi

text = f'The value of Pi is approx. {pi}'

def foo(a: str = None):
    return text

# multi line
# comment

def main():
    print(foo())  # comment


class A:
    a = 123

    def bar(self):
        return foo()


def decorate(f):
    return f


@decorate
def baz():
    pass


if __name__ == '__main__':
    def cond():
        pass

    main()
