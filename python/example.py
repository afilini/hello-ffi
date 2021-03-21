# from bdk.bitcoin import *
# 
# s = Script('a91457d6b4ded38193013643b03b4472e15f80bc465787') 
# a = Address.from_script(s, Network('testnet'))
# 
# print('Address: {}'.format(a.to_string()))
# print('Script: {}'.format(a.script.to_hex()))

from bdk.test_mod import *

a = impl_my_trait_new(42)
use_trait(a)

class PythonClass(MyTraitStruct):
    def __init__(self):
        self.python = self

    def rust_method(self, s):
        print('Printing from Python: {}'.format(s))
        return 'String from Python'

b = PythonClass()
use_trait(b)
