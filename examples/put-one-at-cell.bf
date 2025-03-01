,------------------------------------------------ # load a number into the memory
[
  -         # decrement the number
  [->+<]    # copy the number to the next cell
  >         # move to the next cell
]
+           # increment the number, so we get 1 at the end