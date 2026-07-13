THRESHOLD ?= 0.1
SCALE ?= 1
# SCALE ?= 180
INTRINSIC ?= '886.809983634957,0,512.0,0,886.8099836349569,384.0,0,0,1'
VIEW1_POSE ?= '0.741194,-0.351180,0.572105,2.622013,0.579402,-0.095712,-0.809400,-2.977569,0.339002,0.931403,0.132533,1.582464,0.000000,0.000000,0.000000,1.000000'
VIEW2_POSE ?= '0.749216,-0.279027,0.600683,2.187780,0.650863,0.142168,-0.745765,-2.726643,0.122691,0.949701,0.288123,1.756006,0.000000,0.000000,0.000000,1.000000'

default:
	$(MAKE) single-view

single-view:
	depthmesh -d ./depth/cam_00_frame.0000.exr -n ./normal/cam_00_frame.0000.exr -i $(INTRINSIC) -t $(THRESHOLD) -s $(SCALE) --optimize --smooth -D

multi-view:
	depthmesh \
		-d ./depth/cam_00_frame.0000.exr -n ./normal/cam_00_frame.0000.exr -i $(INTRINSIC) -t $(THRESHOLD) -s $(SCALE) -p $(VIEW1_POSE) \
		-d ./depth/cam_00_frame.0001.exr -n ./normal/cam_00_frame.0001.exr -i $(INTRINSIC) -t $(THRESHOLD) -s $(SCALE) -p $(VIEW2_POSE) \
		-D --optimize --smooth

.PHONY: default single-view multi-view
