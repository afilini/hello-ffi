from bdk.bitcoin import *

s = Script.from_hex('a91457d6b4ded38193013643b03b4472e15f80bc465787') 
a = Address.from_script(s, Network('testnet'))

print('Address: {}'.format(a.to_string()))
print('Script: {}'.format(a.script.to_hex()))

txin = transaction.TxIn(transaction.OutPoint('cdae07af68bc5fab2f294acfa6dc2f9b399ce542901d52aa65d08c8ff3337c48:42'), Script.empty(), 0xFFFFFFFF, [[]])
txout = transaction.TxOut(a.script, int(0.42 * 1e8))
tx = transaction.Transaction(1, 0, [txin], [txout])

print('Transaction: {}'.format(tx.to_hex()))
