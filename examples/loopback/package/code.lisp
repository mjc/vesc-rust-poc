(import "src/package_lib.bin" 'package-lib)
(print "vesc-rust-load-v7")
(print (load-native-lib package-lib))
