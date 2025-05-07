this app is consol app disined to manige multiple prosseses that run perioticlie.

the main app and dll's should be compiled with the same compiler version i use version = []

you can coplite the exe with feature flag "led" and "GPOI" if you what to use the LED feature, if you dont use the GPIO flage it will print the led canges to the console ( used for testing ). if you use the GPIO feature it is ecpected that the exe is run on a raspary py with GPIO and i2c aneblede and that a pca9685 chip is hock up, [how to hock up]
there a five led to show status LED0 is reserved for the main thread the you can use it but it is not recomended, the rest can be use how you wich