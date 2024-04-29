function validateResponse(response) {
    if (response.status === 200) {
        console.log(response.body)
    }
    if (response.status === 400) {
        console.log('404 error')
    }
}

