#include <stdio.h>
#include <assert.h>

int Fallible_func(int value)
{
  assert(value > 0);
  return value;
}

void Print(char *str)
{
  // printf("%s\n", str);
}
