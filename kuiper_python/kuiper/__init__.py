"""
Kuiper is a JSON to JSON transform and templating language from Cognite.

.. code-block:: python

    from kuiper import compile_expression

    expression = compile_expression('{"theAnswer": input.numericValue + 27}', ["input"])
    value = expression.run('{"numericValue": 15}')
    print(value)

The ``compile_expression`` function might raise a ``KuiperCompileError``, and otherwise returns a ``KuiperExpression``
object. The ``KuiperExpression.run(...)`` method might raise a ``KuiperRuntimeError``. Both of these exceptions are
subclasses of the ``KuiperError`` base class.
"""

# from typing import Optional

# class KuiperError(Exception):
#     start: Optional[int]
#     end: Optional[int]
# 
#     def __init__(self, start: Optional[int], end: Optional[int], *args: object) -> None:
#         super().__init__(*args)
#         self.start = start
#         self.end = end

from .kuiper import *
