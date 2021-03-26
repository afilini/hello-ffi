from bdk.test_mod import *

i = Inner(10)
o = Outer(i)

print(o.inner.val)
o.inner.val *= 5
print(o.inner.val)

i2 = Inner(1000)
o.inner = i2
print(o.inner.val)
