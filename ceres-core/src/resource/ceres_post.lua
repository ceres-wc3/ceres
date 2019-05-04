-- ceres post-script start

ceres.__oldMain = main
ceres.__oldConfig = config

function main()
    __ceresMain()
end

function config()
    __ceresConfig()
end

ceres.catch(require("main"))

-- ceres post-script end