current_dir="$(pwd)"
git clone https://github.com/OpenXiangShan/XiangShan
cd XiangShan
git checkout 533652847a406411557b4b8cd1b7c3e7f3712ca6
# To mitigate a XiangShan upstream bug: they renamed the repo from utility to Utility
git config -f .gitmodules submodule.utility.url https://github.com/OpenXiangShan/Utility
git submodule sync utility
make init
cd rocket-chip/
git am ${current_dir}/flashpoint-patches/0001-add-anchor-encoding.patch 
cd ../
cd utility
git am ${current_dir}/flashpoint-patches/0001-add-anchor-logic.patch
cd ..
git am ${current_dir}/flashpoint-patches/0001-anchor-inst-done.patch
git am ${current_dir}/flashpoint-patches/0003-RegNext.patch
git am ${current_dir}/flashpoint-patches/0001-push-missing-modifications.patch
make verilog CONFIG=MinimalConfig  -j16

