from bdk.bitcoin import *

s = Script('a91457d6b4ded38193013643b03b4472e15f80bc465787') 
a = Address.from_script(s, Network('testnet'))

print('Address: {}'.format(a.to_string()))
print('Script: {}'.format(a.script.to_hex()))
