#include <cstdint>
#include <cstdlib>
#include <iostream>

#include <re2/re2.h>

int main(int argc, char **argv) {
  std::size_t iterations = 1000000;
  if (argc > 1) {
    iterations = static_cast<std::size_t>(std::strtoull(argv[1], nullptr, 10));
  }

  re2::RE2 re("needle", re2::RE2::Quiet);
  std::uint64_t matches = 0;
  for (std::size_t i = 0; i < iterations; ++i) {
    const char *text = (i & 1u) == 0 ? "hay needle stack" : "haystack";
    if (re2::RE2::PartialMatch(text, re)) {
      ++matches;
    }
  }

  std::cout << matches << "\n";
  return 0;
}
