from hello import *

print('Result: {}'.format(HelloStruct.hello_static('World!')))

s = HelloStruct('Python init str')
print('Result: {}'.format(s.hello_method('StructWorld!')))

ret = test_pure_fn(['AAAAAAAAAAAAAAHHHHHHHHHH', 'BBBBBBBBBBBBBBBBB'])
print('String returned: {}'.format(ret))

def cb(s, arr, v):
    print('Printing from Python: {} {}'.format(s, v))
    for a in arr:
        print('> {}'.format(a))

    return 'Hello from Python!'

test_callback(cb)

