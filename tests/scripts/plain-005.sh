# test ENV inside command sup
echo $(echo foo bar | awk '{print $NF}')
echo $(echo foo bar z | awk '{print $NF}')
echo `echo foo bar1 | awk '{print $NF}'`
echo `echo foo bar z2 | awk '{print $NF}'`

VER1=`echo foo bar baz | awk '{print $NF}'`; echo $VER1
VER2=$(echo foo bar baz3 | awk '{print $NF}'); echo $VER2
VER3=$(echo foo bar baz4 | awk "{print $NF}"); echo $VER3
VER4=`echo foo bar baz5 | awk '{print $NF}'`; echo $VER4
VER5=`echo foo bar baz6 | awk "{print $NF}"`; echo $VER5
