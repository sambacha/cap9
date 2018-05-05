
const fs = require("fs")
const path = require("path")
const assert = require("assert")
const Web3 = require("web3")

// Connect to our local node
const web3 = new Web3(new Web3.providers.HttpProvider("http://localhost:8545"));

async function Setup() {
    // NOTE: if you run Kovan node there should be an address you've got in the "Option 2: Run Kovan node" step
    web3.eth.defaultAccount = await web3.eth.getAccounts().then(a => a[0]);
    // read JSON ABI
    const abi = JSON.parse(fs.readFileSync(path.resolve(__dirname, "../target/json/KernelContract.json")));
    // convert Wasm binary to hex format
    const codeHex = '0x' + fs.readFileSync(path.resolve(__dirname, "../target/beaker_core.wasm")).toString('hex');

    const KernelContract = new web3.eth.Contract(abi, { data: codeHex, from: web3.eth.defaultAccount });
    
    const KernelDeployTransaction = KernelContract.deploy({ data: codeHex, arguments: [10000000] });
    
    // Creates and Deploys a new Kernel Instance
    return async function newKernel() {
        try {
            await web3.eth.personal.unlockAccount(web3.eth.defaultAccount, "user")
        } catch (e) {
            console.error(e)
        }
        const gas = await KernelDeployTransaction.estimateGas()
    
        // Will create KernelContract with `totalSupply` = 10000000 and print a result
        return KernelDeployTransaction.send({ gasLimit: gas, from: web3.eth.defaultAccount })
    }
}

describe('Kernel', function () {
    
    let newKernel;
    before(async function () {
        newKernel = await Setup();
    })

    describe('constructor', function () {
        it('should have valid address', async function () {
            let kernel = await newKernel();
            let address = kernel.options.address;
            assert(web3.utils.isHex(address))
        })
    })

    describe('.version', function () {
        it('should default to 0', async function () {
            let kernel = await newKernel();        
            let version = await kernel.methods.version().call()
            assert.equal(version, 1)
        })
    })

})