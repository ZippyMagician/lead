-- adapted from https://saltysylvi.github.io/blog/parsing-nested-lists-in-bqn.html

d:'0'+}~10
T:{{y?}(|','=y)*+\+(y,:'[]')|>&:(_1|.)` y,:d}}
ParseVal:{{('['=":>y):ParseLst y)ParseNum y}}
ParseNum:{{(1}y),`10&*&:`+/|.+'0'-`":>y}}
ParseLst:{{
rslt:0#0
[val:rest]:y
cond:{{']'=":>rest}}
(|cond 0):{{
    [val:rest]:ParseVal rest
    rslt:rslt,val
}}".cond 0	(<rslt),1}rest -- there's no while, only do until
(<rslt),1}rest
}}

-- a:'[999,2,[],[31,4167]]'
