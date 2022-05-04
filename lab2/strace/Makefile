CXX = g++
CFLAGS = -std=c++20 -O2 -Wall 

TARGET = strace
OBJS = strace.cpp

all: $(OBJS)
	$(CXX) $(CFLAGS) $(OBJS) -o $(TARGET)

clean:
	rm $(TARGET)