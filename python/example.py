from hello import *

print('Result: {}'.format(HelloStruct.hello_static('World!')))

s = HelloStruct('Python init str')
print('Result: {}'.format(s.hello_method('StructWorld!')))

test_pure_fn('AAAAAAAAAAAAAAHHHHHHHHHH')

